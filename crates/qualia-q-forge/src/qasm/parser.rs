use crate::qasm::{QasmProgram, QasmStatement};
use qualia_core_db::q_hash;

/// Zero-allocation OpenQASM 3 Parser.
pub struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<QasmProgram, &'static str> {
        let mut program = QasmProgram::new();
        self.skip_whitespace();
        
        while self.pos < self.input.len() {
            let statement = self.parse_statement()?;
            program.push(statement)?;
            self.skip_whitespace();
        }
        
        Ok(program)
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let c = self.input[self.pos..].chars().next().unwrap();
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else if self.input[self.pos..].starts_with("//") {
                // Skip comment
                while self.pos < self.input.len() && !self.input[self.pos..].starts_with('\n') {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn parse_statement(&mut self) -> Result<QasmStatement, &'static str> {
        if self.starts_with_keyword("OPENQASM") {
            self.parse_version()?;
            return Ok(QasmStatement::Empty);
        } else if self.starts_with_keyword("include") {
            self.parse_include()?;
            return Ok(QasmStatement::Empty);
        } else if self.starts_with_keyword("qubit") {
            return self.parse_qubit_decl();
        } else if self.starts_with_keyword("gate") {
            return self.parse_gate_decl();
        }
        
        // Try parsing a gate call
        if let Some(ident) = self.parse_identifier() {
            return self.parse_gate_call(ident);
        }

        Err("Unexpected token or unsupported statement")
    }

    fn starts_with_keyword(&self, kw: &str) -> bool {
        if self.input[self.pos..].starts_with(kw) {
            let next_char = self.input[self.pos + kw.len()..].chars().next();
            match next_char {
                Some(c) if c.is_alphanumeric() || c == '_' => false,
                _ => true,
            }
        } else {
            false
        }
    }

    fn parse_version(&mut self) -> Result<(), &'static str> {
        self.pos += "OPENQASM".len();
        self.skip_whitespace();
        // Parse version number (e.g., "3.0")
        while self.pos < self.input.len() && !self.input[self.pos..].starts_with(';') {
            self.pos += 1;
        }
        self.expect_char(';')?;
        Ok(())
    }

    fn parse_include(&mut self) -> Result<(), &'static str> {
        self.pos += "include".len();
        self.skip_whitespace();
        // Skip string literal
        if self.input[self.pos..].starts_with('"') {
            self.pos += 1;
            while self.pos < self.input.len() && !self.input[self.pos..].starts_with('"') {
                self.pos += 1;
            }
            if self.pos < self.input.len() {
                self.pos += 1; // Skip closing quote
            }
        }
        self.skip_whitespace();
        self.expect_char(';')?;
        Ok(())
    }

    fn parse_qubit_decl(&mut self) -> Result<QasmStatement, &'static str> {
        self.pos += "qubit".len();
        self.skip_whitespace();
        
        let mut size = 1;
        if self.input[self.pos..].starts_with('[') {
            self.pos += 1;
            size = self.parse_u16()?;
            self.expect_char(']')?;
        }
        
        self.skip_whitespace();
        let name = self.parse_identifier().ok_or("Expected qubit name")?;
        self.skip_whitespace();
        self.expect_char(';')?;
        
        Ok(QasmStatement::QubitDecl {
            name_hash: q_hash(name),
            size,
        })
    }

    fn parse_gate_decl(&mut self) -> Result<QasmStatement, &'static str> {
        self.pos += "gate".len();
        self.skip_whitespace();
        let name = self.parse_identifier().ok_or("Expected gate name")?;
        self.skip_whitespace();
        
        // Skip parameters if present (e.g., gate rx(theta) q { ... })
        if self.input[self.pos..].starts_with('(') {
            self.pos += 1;
            while self.pos < self.input.len() && !self.input[self.pos..].starts_with(')') {
                self.pos += 1;
            }
            self.pos += 1; // Skip ')'
            self.skip_whitespace();
        }
        
        // Skip qubit arguments
        let mut num_qubits = 0;
        while self.pos < self.input.len() && !self.input[self.pos..].starts_with('{') {
            if self.parse_identifier().is_some() {
                num_qubits += 1;
            }
            self.skip_whitespace();
            if self.input[self.pos..].starts_with(',') {
                self.pos += 1;
                self.skip_whitespace();
            }
        }
        
        self.expect_char('{')?;
        
        // Skip body for now (stub implementation)
        while self.pos < self.input.len() && !self.input[self.pos..].starts_with('}') {
            self.pos += 1;
        }
        self.expect_char('}')?;
        
        Ok(QasmStatement::GateDecl {
            name_hash: q_hash(name),
            num_qubits,
        })
    }

    fn parse_gate_call(&mut self, name: &'a str) -> Result<QasmStatement, &'static str> {
        self.skip_whitespace();
        
        let mut params = [0.0; 4];
        let mut num_params = 0;
        
        if self.input[self.pos..].starts_with('(') {
            self.pos += 1;
            self.skip_whitespace();
            while self.pos < self.input.len() && !self.input[self.pos..].starts_with(')') {
                if num_params >= 4 {
                    return Err("Too many parameters");
                }
                params[num_params as usize] = self.parse_f64()?;
                num_params += 1;
                self.skip_whitespace();
                if self.input[self.pos..].starts_with(',') {
                    self.pos += 1;
                    self.skip_whitespace();
                }
            }
            self.expect_char(')')?;
            self.skip_whitespace();
        }
        
        let mut target_qubits = [0; 4];
        let mut num_targets = 0;
        
        while self.pos < self.input.len() && !self.input[self.pos..].starts_with(';') {
            if num_targets >= 4 {
                return Err("Too many targets");
            }
            let _qubit_name = self.parse_identifier().ok_or("Expected qubit name")?;
            self.skip_whitespace();
            
            let mut index = 0;
            if self.input[self.pos..].starts_with('[') {
                self.pos += 1;
                index = self.parse_u16()?;
                self.expect_char(']')?;
                self.skip_whitespace();
            }
            
            target_qubits[num_targets as usize] = index;
            num_targets += 1;
            
            if self.input[self.pos..].starts_with(',') {
                self.pos += 1;
                self.skip_whitespace();
            } else {
                break;
            }
        }
        
        self.expect_char(';')?;
        
        Ok(QasmStatement::GateCall {
            name_hash: q_hash(name),
            target_qubits,
            num_targets,
            params,
            num_params,
        })
    }

    fn parse_identifier(&mut self) -> Option<&'a str> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let c = self.input[self.pos..].chars().next().unwrap();
            if c.is_alphanumeric() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        if self.pos > start {
            Some(&self.input[start..self.pos])
        } else {
            None
        }
    }

    fn parse_u16(&mut self) -> Result<u16, &'static str> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let c = self.input[self.pos..].chars().next().unwrap();
            if c.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos > start {
            self.input[start..self.pos].parse().map_err(|_| "Invalid u16")
        } else {
            Err("Expected number")
        }
    }

    fn parse_f64(&mut self) -> Result<f64, &'static str> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let c = self.input[self.pos..].chars().next().unwrap();
            if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos > start {
            // NOTE: In #![no_std], f64 parsing might be tricky if we don't have alloc.
            // core::str::parse::<f64> exists and is usable.
            self.input[start..self.pos].parse().map_err(|_| "Invalid f64")
        } else {
            Err("Expected number")
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), &'static str> {
        if self.input[self.pos..].starts_with(expected) {
            self.pos += expected.len_utf8();
            Ok(())
        } else {
            Err("Unexpected character")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let source = "
            OPENQASM 3.0;
            include \"stdgates.inc\";
            qubit[2] q;
            h q[0];
            cx q[0], q[1];
        ";
        let mut parser = Parser::new(source);
        let program = parser.parse().unwrap();
        assert_eq!(program.statement_count, 5);
        
        match program.statements[2] {
            QasmStatement::QubitDecl { size, .. } => assert_eq!(size, 2),
            _ => panic!("Expected QubitDecl"),
        }
    }
}
