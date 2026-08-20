//! A2 — Speculative Constrained Decoding (DOMINO).
//!
//! Subword-aligned prefix-trie token masking integrated into the in-process
//! `QTensorEngine` decode loop. The approach:
//!
//! 1. **TokenTrie** — a prefix trie over token byte strings, built once from
//!    the tokenizer vocabulary (cold path).
//! 2. **GrammarState** — a state machine tracking which character classes and
//!    keywords are valid at the current parse position, based on the VibeScript
//!    0.1 GBNF grammar.
//! 3. **DominoMasker** — combines the trie and grammar state to produce a logit
//!    mask, setting disallowed tokens to `-inf` before sampling.
//!
//! The mask is applied per-token on the hot path using caller-supplied buffers
//! (zero-heap). The trie is walked incrementally: at each decode step, the
//! grammar state determines valid byte ranges, and the trie is traversed to
//! find all token IDs whose byte sequences are valid continuations.
//!
//! ## Integration
//!
//! The masker is called between logit readback and the sampler:
//! ```text
//! logits = engine.readback_logits();
//! masker.apply_mask(&mut logits, &accepted_tokens);
//! token = sampler.sample(&mut logits, &ctx);
//! ```
//!
//! When no grammar is active (unconstrained mode), `apply_mask` is a no-op.

use std::collections::HashMap;

// ── Token Trie ──────────────────────────────────────────────────────────────

/// Maximum trie depth (longest token in bytes we support).
pub const MAX_TRIE_DEPTH: usize = 32;

/// A node in the token prefix trie.
#[derive(Debug, Default)]
struct TrieNode {
    /// Children keyed by byte value.
    children: HashMap<u8, TrieNode>,
    /// Token IDs that terminate at this node (a token's bytes end here).
    token_ids: Vec<u32>,
}

/// A prefix trie over token byte strings. Built once from the vocabulary.
pub struct TokenTrie {
    root: TrieNode,
    vocab_size: usize,
}

impl TokenTrie {
    /// Build a trie from a vocabulary: `(token_id, token_bytes)` pairs.
    /// Cold path — allocation is expected here.
    pub fn build(vocab: &[(u32, &[u8])]) -> Self {
        let mut root = TrieNode::default();
        let vocab_size = vocab.len();
        for &(id, bytes) in vocab {
            let mut node = &mut root;
            for &b in bytes.iter().take(MAX_TRIE_DEPTH) {
                node = node.children.entry(b).or_default();
            }
            node.token_ids.push(id);
        }
        Self { root, vocab_size }
    }

    /// Build from a tokenizer that provides `token_id → string` lookups.
    pub fn build_from_strings(vocab: &[(u32, String)]) -> Self {
        let refs: Vec<(u32, &[u8])> = vocab.iter().map(|(id, s)| (*id, s.as_bytes())).collect();
        Self::build(&refs)
    }

    /// Walk the trie from the root, collecting all token IDs reachable via
    /// bytes that pass the `byte_allowed` predicate. This is the core mask
    /// computation: a token is valid iff every byte in its string passes
    /// the predicate at the corresponding depth.
    ///
    /// Hot path — uses caller-supplied output buffer (zero-heap).
    pub fn collect_valid(
        &self,
        byte_allowed: &dyn Fn(u8, usize) -> bool,
        out: &mut [u32],
    ) -> usize {
        let mut count = 0;
        self.walk(
            &self.root,
            byte_allowed,
            &mut [0u8; MAX_TRIE_DEPTH],
            0,
            out,
            &mut count,
        );
        count
    }

    fn walk(
        &self,
        node: &TrieNode,
        byte_allowed: &dyn Fn(u8, usize) -> bool,
        path: &mut [u8; MAX_TRIE_DEPTH],
        depth: usize,
        out: &mut [u32],
        count: &mut usize,
    ) {
        // Emit token IDs at this node.
        for &id in &node.token_ids {
            if *count >= out.len() {
                return;
            }
            out[*count] = id;
            *count += 1;
        }
        if depth >= MAX_TRIE_DEPTH {
            return;
        }
        for (&b, child) in &node.children {
            if byte_allowed(b, depth) {
                path[depth] = b;
                self.walk(child, byte_allowed, path, depth + 1, out, count);
            }
        }
    }

    /// Number of tokens in the vocabulary.
    pub fn vocab_size(&self) -> usize {
        self.vocab_size
    }
}

// ── Grammar State ───────────────────────────────────────────────────────────

/// VibeScript 0.1 grammar parse states. These correspond to the non-terminals
/// in `vibe-0.1.gbnf` that determine which characters are valid next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammarState {
    /// Start of input — expects a cell (`=`) or module keyword.
    Start,
    /// After `=` — expects an expression.
    CellExpr,
    /// Expecting a keyword: `module`, `import`, `prefix`, `requires`, `fn`,
    /// `on`, `let`, `return`, or an identifier (statement/expression).
    KeywordOrIdent,
    /// Inside whitespace — any amount of whitespace is consumed.
    Whitespace,
    /// Inside an identifier — `[A-Za-z0-9_]` continue, anything else ends it.
    Ident,
    /// Inside a number — `[0-9.]` continue.
    Number,
    /// Inside a string literal — `[^"\\]` or escape sequences continue.
    String,
    /// Inside an IRI — `[^<>\s]` continue.
    Iri,
    /// Inside a block — `{` entered, statements expected.
    Block,
    /// Inside a triple — `<<(` entered, three expressions expected.
    Triple,
    /// Inside a reified triple — `<<` entered.
    Reified,
    /// Inside a list — `[` entered.
    List,
    /// Inside a parenthesized expression.
    Paren,
    /// Unconstrained — no grammar active (free generation).
    Unconstrained,
}

impl Default for GrammarState {
    fn default() -> Self {
        Self::Start
    }
}

/// The grammar state machine. Tracks the current parse position and a stack
/// of nested contexts (blocks, parens, triples, etc.) for proper nesting.
pub struct GrammarStateMachine {
    /// Current state.
    state: GrammarState,
    /// Stack of nested contexts (for closing braces/brackets/parens).
    stack: Vec<GrammarState>,
    /// The decoded text so far (for keyword matching).
    text: String,
    /// The state to return to after whitespace is consumed. This preserves
    /// the semantic context (e.g. CellExpr, Block) across whitespace.
    whitespace_parent: GrammarState,
}

impl GrammarStateMachine {
    pub fn new() -> Self {
        Self {
            state: GrammarState::Start,
            stack: Vec::new(),
            text: String::new(),
            whitespace_parent: GrammarState::Start,
        }
    }

    /// Reset to initial state.
    pub fn reset(&mut self) {
        self.state = GrammarState::Start;
        self.stack.clear();
        self.text.clear();
        self.whitespace_parent = GrammarState::Start;
    }

    /// Feed a decoded byte into the state machine, updating the state.
    pub fn feed_byte(&mut self, b: u8) {
        self.text.push(b as char);
        self.state = self.transition(b);
    }

    /// Feed a decoded token's bytes.
    pub fn feed_token(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.feed_byte(b);
        }
    }

    /// Determine the next state given the current state and a byte.
    fn transition(&mut self, b: u8) -> GrammarState {
        use GrammarState::*;
        let c = b as char;

        // Whitespace is handled uniformly: save the semantic context and
        // skip. When a non-whitespace byte arrives after whitespace, the
        // `Whitespace` state dispatches based on `whitespace_parent`.
        if c.is_whitespace() && self.state != Whitespace {
            self.whitespace_parent = self.state;
            return Whitespace;
        }

        match self.state {
            Unconstrained => Unconstrained,
            Start => {
                if c == '=' {
                    CellExpr
                } else if c.is_alphabetic() || c == '_' {
                    self.text.clear();
                    self.text.push(c);
                    Ident
                } else {
                    Unconstrained
                }
            }
            Whitespace => {
                if c.is_whitespace() {
                    Whitespace
                } else {
                    // Dispatch based on the semantic context before whitespace.
                    self.dispatch_after_whitespace(c)
                }
            }
            CellExpr => {
                if c.is_ascii_digit() || c == '-' {
                    Number
                } else if c == '"' {
                    String
                } else if c == '<' {
                    Iri
                } else if c == '[' {
                    self.stack.push(List);
                    List
                } else if c == '(' {
                    self.stack.push(Paren);
                    Paren
                } else if c.is_alphabetic() || c == '_' || c == '?' {
                    self.text.clear();
                    self.text.push(c);
                    Ident
                } else {
                    Unconstrained
                }
            }
            Ident => {
                if c.is_alphanumeric() || c == '_' || c == '.' || c == '-' || c == ':' {
                    Ident
                } else if c == '{' {
                    self.stack.push(Block);
                    Block
                } else if c == '(' {
                    self.stack.push(Paren);
                    Paren
                } else if c == ';' {
                    KeywordOrIdent
                } else {
                    Unconstrained
                }
            }
            Number => {
                if c.is_ascii_digit() || c == '.' {
                    Number
                } else if c == ';' {
                    KeywordOrIdent
                } else {
                    Unconstrained
                }
            }
            String => {
                if c == '"' {
                    // String ended — return to the context that contained it.
                    if let Some(parent) = self.stack.last().copied() {
                        parent
                    } else {
                        KeywordOrIdent
                    }
                } else {
                    String
                }
            }
            Iri => {
                if c == '>' {
                    if let Some(parent) = self.stack.last().copied() {
                        parent
                    } else {
                        KeywordOrIdent
                    }
                } else {
                    Iri
                }
            }
            Block => {
                if c == '}' {
                    self.stack.pop();
                    KeywordOrIdent
                } else if c.is_alphabetic() || c == '_' {
                    self.text.clear();
                    self.text.push(c);
                    Ident
                } else {
                    Unconstrained
                }
            }
            Triple | Reified => self.transition_generic(c),
            List => {
                if c == ']' {
                    self.stack.pop();
                    if let Some(parent) = self.stack.last().copied() {
                        parent
                    } else {
                        KeywordOrIdent
                    }
                } else {
                    self.transition_generic(c)
                }
            }
            Paren => {
                if c == ')' {
                    self.stack.pop();
                    if let Some(parent) = self.stack.last().copied() {
                        parent
                    } else {
                        KeywordOrIdent
                    }
                } else {
                    self.transition_generic(c)
                }
            }
            KeywordOrIdent => {
                if c.is_alphabetic() || c == '_' {
                    self.text.clear();
                    self.text.push(c);
                    Ident
                } else if c == '}' {
                    self.stack.pop();
                    KeywordOrIdent
                } else {
                    Unconstrained
                }
            }
        }
    }

    /// Dispatch after whitespace, based on the semantic context that preceded it.
    fn dispatch_after_whitespace(&mut self, c: char) -> GrammarState {
        use GrammarState::*;
        match self.whitespace_parent {
            Start | KeywordOrIdent => {
                if c == '=' {
                    CellExpr
                } else if c == '{' {
                    self.stack.push(Block);
                    Block
                } else if c.is_alphabetic() || c == '_' {
                    self.text.clear();
                    self.text.push(c);
                    Ident
                } else {
                    Unconstrained
                }
            }
            CellExpr => {
                if c.is_ascii_digit() || c == '-' {
                    Number
                } else if c == '"' {
                    String
                } else if c == '<' {
                    Iri
                } else if c == '[' {
                    self.stack.push(List);
                    List
                } else if c == '(' {
                    self.stack.push(Paren);
                    Paren
                } else if c.is_alphabetic() || c == '_' || c == '?' {
                    self.text.clear();
                    self.text.push(c);
                    Ident
                } else {
                    Unconstrained
                }
            }
            Block => {
                if c == '}' {
                    self.stack.pop();
                    KeywordOrIdent
                } else if c.is_alphabetic() || c == '_' {
                    self.text.clear();
                    self.text.push(c);
                    Ident
                } else {
                    Unconstrained
                }
            }
            Ident => {
                // After an identifier + whitespace, `(` (call/params), `{` (block),
                // `;` (statement end) are valid.
                if c == '(' {
                    self.stack.push(Paren);
                    Paren
                } else if c == '{' {
                    self.stack.push(Block);
                    Block
                } else if c == ';' {
                    KeywordOrIdent
                } else if c.is_alphabetic() || c == '_' {
                    self.text.clear();
                    self.text.push(c);
                    Ident
                } else {
                    Unconstrained
                }
            }
            _ => self.transition_generic(c),
        }
    }

    fn transition_generic(&mut self, c: char) -> GrammarState {
        if c.is_ascii_digit() || c == '-' {
            GrammarState::Number
        } else if c == '"' {
            GrammarState::String
        } else if c == '<' {
            GrammarState::Iri
        } else if c.is_alphabetic() || c == '_' || c == '?' {
            self.text.clear();
            self.text.push(c);
            GrammarState::Ident
        } else {
            GrammarState::Unconstrained
        }
    }

    /// Get the current grammar state.
    pub fn state(&self) -> GrammarState {
        self.state
    }

    /// Is the grammar currently active (not unconstrained)?
    pub fn is_active(&self) -> bool {
        self.state != GrammarState::Unconstrained
    }

    /// Determine if a byte is allowed at the current grammar position.
    /// This is the predicate used by the trie walker.
    pub fn byte_allowed(&self, b: u8, _depth: usize) -> bool {
        use GrammarState::*;
        let c = b as char;
        match self.state {
            Unconstrained => true,
            Whitespace => {
                // In whitespace, allowed bytes depend on the parent context.
                if c.is_whitespace() {
                    return true;
                }
                self.byte_allowed_parent(b)
            }
            Start => c.is_whitespace() || c == '=' || c.is_alphabetic() || c == '_',
            CellExpr => {
                c.is_whitespace()
                    || c.is_ascii_digit()
                    || c == '-'
                    || c == '"'
                    || c == '<'
                    || c == '['
                    || c == '('
                    || c.is_alphabetic()
                    || c == '_'
                    || c == '?'
            }
            Ident => {
                c.is_alphanumeric()
                    || c == '_'
                    || c == '.'
                    || c == '-'
                    || c == ':'
                    || c.is_whitespace()
                    || c == '{'
                    || c == '('
                    || c == ';'
            }
            Number => c.is_ascii_digit() || c == '.' || c.is_whitespace() || c == ';',
            String => true, // Any byte can appear inside a string (escape sequences).
            Iri => b != b'>' && !c.is_whitespace() || b == b'>',
            Block => c.is_whitespace() || c == '}' || c.is_alphabetic() || c == '_',
            List | Paren | Triple | Reified => {
                c.is_whitespace()
                    || c.is_ascii_digit()
                    || c == '-'
                    || c == '"'
                    || c == '<'
                    || c == '['
                    || c == '('
                    || c == ']'
                    || c == ')'
                    || c.is_alphabetic()
                    || c == '_'
                    || c == '?'
            }
            KeywordOrIdent => {
                c.is_whitespace() || c.is_alphabetic() || c == '_' || c == '}' || c == '{'
            }
        }
    }

    /// Check if a byte is allowed based on the whitespace parent context.
    fn byte_allowed_parent(&self, b: u8) -> bool {
        use GrammarState::*;
        let c = b as char;
        match self.whitespace_parent {
            Start | KeywordOrIdent => c == '=' || c == '{' || c.is_alphabetic() || c == '_',
            CellExpr => {
                c.is_ascii_digit()
                    || c == '-'
                    || c == '"'
                    || c == '<'
                    || c == '['
                    || c == '('
                    || c.is_alphabetic()
                    || c == '_'
                    || c == '?'
            }
            Block => c == '}' || c.is_alphabetic() || c == '_',
            Ident => c == '(' || c == '{' || c == ';' || c.is_alphabetic() || c == '_',
            _ => {
                c.is_ascii_digit()
                    || c == '-'
                    || c == '"'
                    || c == '<'
                    || c == '['
                    || c == '('
                    || c == ']'
                    || c == ')'
                    || c.is_alphabetic()
                    || c == '_'
                    || c == '?'
            }
        }
    }
}

impl Default for GrammarStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

// ── DOMINO Masker ───────────────────────────────────────────────────────────

/// The DOMINO constrained decoding masker. Combines a token trie with a
/// grammar state machine to produce logit masks.
pub struct DominoMasker {
    trie: TokenTrie,
    grammar: GrammarStateMachine,
    /// Scratch buffer for valid token IDs (reused across calls — zero-heap on hot path).
    valid_buf: Vec<u32>,
    /// Whether constrained decoding is active.
    active: bool,
}

impl DominoMasker {
    /// Create a new masker from a vocabulary. The trie is built once (cold path).
    pub fn new(vocab: &[(u32, String)]) -> Self {
        Self {
            trie: TokenTrie::build_from_strings(vocab),
            grammar: GrammarStateMachine::new(),
            valid_buf: Vec::with_capacity(4096),
            active: false,
        }
    }

    /// Enable constrained decoding.
    pub fn enable(&mut self) {
        self.active = true;
        self.grammar.reset();
    }

    /// Disable constrained decoding (free generation).
    pub fn disable(&mut self) {
        self.active = false;
    }

    /// Is constrained decoding active?
    pub fn is_active(&self) -> bool {
        self.active && self.grammar.is_active()
    }

    /// Apply the constraint mask to logits. Sets disallowed tokens to `-inf`.
    /// When inactive, this is a no-op.
    ///
    /// Hot path — reuses internal scratch buffer, no allocation.
    pub fn apply_mask(&mut self, logits: &mut [f32]) {
        if !self.is_active() {
            return;
        }
        // Collect valid token IDs from the trie.
        self.valid_buf.clear();
        self.valid_buf.resize(logits.len(), 0);
        let grammar = &self.grammar;
        let count = self.trie.collect_valid(
            &|b, depth| grammar.byte_allowed(b, depth),
            &mut self.valid_buf,
        );
        // Mask: set all logits to -inf, then restore valid ones.
        // This is O(vocab) — acceptable at the one-fence-per-token cost.
        for logit in logits.iter_mut() {
            *logit = f32::NEG_INFINITY;
        }
        for i in 0..count {
            let id = self.valid_buf[i] as usize;
            if id < logits.len() {
                logits[id] = 0.0; // Restore to a finite value (original logit is lost).
            }
        }
    }

    /// Apply the constraint mask while preserving original logit values.
    /// Uses a snapshot approach: first save valid logits, then mask all,
    /// then restore valid ones.
    pub fn apply_mask_preserving(&mut self, logits: &mut [f32]) {
        if !self.is_active() {
            return;
        }
        self.valid_buf.clear();
        self.valid_buf.resize(logits.len(), 0);
        let grammar = &self.grammar;
        let count = self.trie.collect_valid(
            &|b, depth| grammar.byte_allowed(b, depth),
            &mut self.valid_buf,
        );
        // Save valid logits, mask all, restore.
        let mut saved = [0f32; 256];
        for i in 0..count.min(saved.len()) {
            let id = self.valid_buf[i] as usize;
            if id < logits.len() {
                saved[i] = logits[id];
            }
        }
        for logit in logits.iter_mut() {
            *logit = f32::NEG_INFINITY;
        }
        for i in 0..count.min(saved.len()) {
            let id = self.valid_buf[i] as usize;
            if id < logits.len() {
                logits[id] = saved[i];
            }
        }
    }

    /// Feed a decoded token into the grammar state machine.
    pub fn feed_token(&mut self, bytes: &[u8]) {
        if self.active {
            self.grammar.feed_token(bytes);
        }
    }

    /// Feed a decoded token ID by looking up its bytes in the vocabulary.
    pub fn feed_token_id(&mut self, token_id: u32, vocab: &[(u32, String)]) {
        if !self.active {
            return;
        }
        for &(id, ref s) in vocab {
            if id == token_id {
                self.grammar.feed_token(s.as_bytes());
                return;
            }
        }
    }

    /// Get the current grammar state.
    pub fn grammar_state(&self) -> GrammarState {
        self.grammar.state()
    }

    /// Reset the grammar state machine (e.g., for a new generation).
    pub fn reset(&mut self) {
        self.grammar.reset();
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vocab() -> Vec<(u32, String)> {
        vec![
            (0, "=".to_string()),
            (1, " ".to_string()),
            (2, "\n".to_string()),
            (3, "module".to_string()),
            (4, "import".to_string()),
            (5, "fn".to_string()),
            (6, "let".to_string()),
            (7, "return".to_string()),
            (8, "x".to_string()),
            (9, "42".to_string()),
            (10, ";".to_string()),
            (11, "\"hello\"".to_string()),
            (12, "{".to_string()),
            (13, "}".to_string()),
            (14, "(".to_string()),
            (15, ")".to_string()),
            (16, "i32".to_string()),
            (17, "=".to_string()),
            (18, "1".to_string()),
            (19, "2".to_string()),
            (20, "test".to_string()),
            (21, "abc".to_string()),
        ]
    }

    #[test]
    fn trie_build_and_lookup() {
        let vocab = test_vocab();
        let trie = TokenTrie::build_from_strings(&vocab);
        assert_eq!(trie.vocab_size(), 22);
        let mut out = [0u32; 64];
        let count = trie.collect_valid(&|_b, _d| true, &mut out);
        assert!(count > 0);
    }

    #[test]
    fn trie_filters_by_byte_predicate() {
        let vocab = test_vocab();
        let trie = TokenTrie::build_from_strings(&vocab);
        let mut out = [0u32; 64];
        // Only allow '=' as first byte.
        let count = trie.collect_valid(&|b, depth| depth == 0 && b == b'=', &mut out);
        assert!(count > 0);
        // All returned tokens should start with '='.
        for i in 0..count {
            assert!(out[i] == 0 || out[i] == 17); // "=" or "="
        }
    }

    #[test]
    fn grammar_state_start() {
        let mut gsm = GrammarStateMachine::new();
        assert_eq!(gsm.state(), GrammarState::Start);
        gsm.feed_byte(b' ');
        assert_eq!(gsm.state(), GrammarState::Whitespace);
        gsm.feed_byte(b'=');
        assert_eq!(gsm.state(), GrammarState::CellExpr);
    }

    #[test]
    fn grammar_state_ident() {
        let mut gsm = GrammarStateMachine::new();
        gsm.feed_byte(b'x');
        assert_eq!(gsm.state(), GrammarState::Ident);
        gsm.feed_byte(b' ');
        // After whitespace, state is Whitespace with parent=Ident.
        assert_eq!(gsm.state(), GrammarState::Whitespace);
        // Feeding another identifier transitions back to Ident.
        gsm.feed_byte(b'y');
        assert_eq!(gsm.state(), GrammarState::Ident);
    }

    #[test]
    fn grammar_state_number() {
        let mut gsm = GrammarStateMachine::new();
        gsm.feed_byte(b'=');
        gsm.feed_byte(b' ');
        // After whitespace, parent is CellExpr → digit starts a number.
        gsm.feed_byte(b'4');
        assert_eq!(gsm.state(), GrammarState::Number);
        gsm.feed_byte(b'2');
        assert_eq!(gsm.state(), GrammarState::Number);
    }

    #[test]
    fn grammar_state_string() {
        let mut gsm = GrammarStateMachine::new();
        gsm.feed_byte(b'=');
        gsm.feed_byte(b' ');
        // After whitespace, parent is CellExpr → `"` starts a string.
        gsm.feed_byte(b'"');
        assert_eq!(gsm.state(), GrammarState::String);
        gsm.feed_byte(b'h');
        assert_eq!(gsm.state(), GrammarState::String);
        gsm.feed_byte(b'"');
        // String ended.
        assert_ne!(gsm.state(), GrammarState::String);
    }

    #[test]
    fn grammar_state_block_nesting() {
        let mut gsm = GrammarStateMachine::new();
        // `fn x () {}`
        gsm.feed_byte(b'f');
        gsm.feed_byte(b'n');
        assert_eq!(gsm.state(), GrammarState::Ident);
        gsm.feed_byte(b' ');
        assert_eq!(gsm.state(), GrammarState::Whitespace);
        gsm.feed_byte(b'x');
        assert_eq!(gsm.state(), GrammarState::Ident);
        gsm.feed_byte(b' ');
        assert_eq!(gsm.state(), GrammarState::Whitespace);
        gsm.feed_byte(b'(');
        assert_eq!(gsm.state(), GrammarState::Paren);
        gsm.feed_byte(b')');
        assert_ne!(gsm.state(), GrammarState::Paren);
        gsm.feed_byte(b' ');
        gsm.feed_byte(b'{');
        assert_eq!(gsm.state(), GrammarState::Block);
        gsm.feed_byte(b'}');
        assert_ne!(gsm.state(), GrammarState::Block);
    }

    #[test]
    fn domino_masker_inactive_noop() {
        let vocab = test_vocab();
        let mut masker = DominoMasker::new(&vocab);
        let mut logits = [1.0f32, 2.0, 3.0, 4.0];
        masker.apply_mask(&mut logits);
        // Inactive — logits unchanged.
        assert_eq!(logits, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn domino_masker_active_masks() {
        let vocab = test_vocab();
        let mut masker = DominoMasker::new(&vocab);
        masker.enable();
        let mut logits = vec![1.0f32; 22];
        masker.apply_mask(&mut logits);
        // At Start state, only tokens starting with '=', whitespace, or alpha
        // should be unmasked.
        let unmasked = logits.iter().filter(|&&l| l > f32::NEG_INFINITY).count();
        assert!(unmasked > 0, "at least some tokens should be valid");
        assert!(unmasked < 22, "not all tokens should be valid at Start");
    }

    #[test]
    fn domino_masker_feed_and_transition() {
        let vocab = test_vocab();
        let mut masker = DominoMasker::new(&vocab);
        masker.enable();
        // Feed '=' token.
        masker.feed_token(b"=");
        assert_eq!(masker.grammar_state(), GrammarState::CellExpr);
        // Now only expression-starting tokens should be valid.
        let mut logits = vec![1.0f32; 22];
        masker.apply_mask(&mut logits);
        let unmasked = logits.iter().filter(|&&l| l > f32::NEG_INFINITY).count();
        assert!(unmasked > 0);
    }

    #[test]
    fn domino_masker_reset() {
        let vocab = test_vocab();
        let mut masker = DominoMasker::new(&vocab);
        masker.enable();
        masker.feed_token(b"=");
        assert_ne!(masker.grammar_state(), GrammarState::Start);
        masker.reset();
        assert_eq!(masker.grammar_state(), GrammarState::Start);
    }

    #[test]
    fn domino_masker_disable() {
        let vocab = test_vocab();
        let mut masker = DominoMasker::new(&vocab);
        masker.enable();
        assert!(masker.is_active());
        masker.disable();
        assert!(!masker.is_active());
        let mut logits = vec![1.0f32; 22];
        masker.apply_mask(&mut logits);
        // All logits unchanged.
        assert!(logits.iter().all(|&l| l == 1.0));
    }

    #[test]
    fn domino_masker_preserving() {
        let vocab = test_vocab();
        let mut masker = DominoMasker::new(&vocab);
        masker.enable();
        let mut logits = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let original = logits.clone();
        masker.apply_mask_preserving(&mut logits);
        // Valid tokens should have their original values, invalid should be -inf.
        let has_neg_inf = logits.iter().any(|&l| l == f32::NEG_INFINITY);
        assert!(has_neg_inf, "some tokens should be masked");
        // At least one token should retain its original value.
        let has_original = logits.iter().any(|&l| l > 0.0);
        assert!(has_original, "some tokens should be unmasked");
    }

    #[test]
    fn byte_allowed_start_state() {
        let gsm = GrammarStateMachine::new();
        // At Start: whitespace, '=', alpha, '_' allowed.
        assert!(gsm.byte_allowed(b' ', 0));
        assert!(gsm.byte_allowed(b'=', 0));
        assert!(gsm.byte_allowed(b'x', 0));
        assert!(gsm.byte_allowed(b'_', 0));
        // Digits, '{', etc. not allowed at Start.
        assert!(!gsm.byte_allowed(b'1', 0));
        assert!(!gsm.byte_allowed(b'{', 0));
    }

    #[test]
    fn byte_allowed_cell_expr_state() {
        let mut gsm = GrammarStateMachine::new();
        gsm.feed_byte(b'=');
        // At CellExpr (before whitespace): digits, strings, idents, etc. allowed.
        assert!(gsm.byte_allowed(b'1', 0));
        assert!(gsm.byte_allowed(b'"', 0));
        assert!(gsm.byte_allowed(b'x', 0));
    }

    #[test]
    fn trie_max_depth_respected() {
        let vocab = vec![(0, "a".repeat(40))];
        let trie = TokenTrie::build_from_strings(&vocab);
        let mut out = [0u32; 10];
        let count = trie.collect_valid(&|_b, _d| true, &mut out);
        // Token longer than MAX_TRIE_DEPTH should still be indexed (truncated).
        assert!(count > 0);
    }

    #[test]
    fn grammar_unconstrained_relax() {
        let mut gsm = GrammarStateMachine::new();
        // Feed an unexpected byte at Start.
        gsm.feed_byte(b'!');
        assert_eq!(gsm.state(), GrammarState::Unconstrained);
        // Once unconstrained, everything is allowed.
        assert!(gsm.byte_allowed(b'x', 0));
        assert!(gsm.byte_allowed(b'!', 0));
    }
}
