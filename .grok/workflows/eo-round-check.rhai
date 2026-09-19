let meta = #{
    name: "eo-round-check",
    description: "Independent round-gate review for one executable-operator swarm wave",
    when_to_use: "After an EO swarm wave's packets are in REVIEW; do not use to implement code",
    phases: [
        #{ title: "Mechanical", detail: "validator, ownership, board status" },
        #{ title: "Review", detail: "one independent reviewer per packet" },
        #{ title: "Gate", detail: "aggregate PASS or FAIL" },
    ],
};

let mechanical_schema = #{
    "type": "object",
    "required": ["validator_ok", "off_limits_clean", "wave_packets_review_or_done", "notes"],
    "properties": #{
        "validator_ok": #{ "type": "boolean" },
        "off_limits_clean": #{ "type": "boolean" },
        "wave_packets_review_or_done": #{ "type": "boolean" },
        "notes": #{ "type": "string" },
    },
};

let packet_schema = #{
    "type": "object",
    "required": ["packet", "verdict", "evidence", "defects"],
    "properties": #{
        "packet": #{ "type": "string" },
        "verdict": #{ "type": "string" },
        "evidence": #{ "type": "string" },
        "defects": #{ "type": "array", "items": #{ "type": "string" }, "maxItems": 12 },
    },
};

let wave = if args == () { () } else { args.wave };
if wave == () {
    pause("verification", "Pass args.wave as W0, W1, W2, W3, W4, W5, W6, W7, W8, or W9.");
}

let packets = [];
if wave == "W0" {
    packets = ["EOS-000", "EOS-001", "EOS-002"];
} else if wave == "W1" {
    packets = ["EOS-010", "EOS-011", "EOS-012", "EOS-013", "EOS-014"];
} else if wave == "W2" {
    packets = ["EOS-020", "EOS-021", "EOS-022", "EOS-023"];
} else if wave == "W3" {
    packets = ["EOS-030", "EOS-031", "EOS-040"];
} else if wave == "W4" {
    packets = ["EOS-041", "EOS-042"];
} else if wave == "W5" {
    packets = ["EOS-050", "EOS-051", "EOS-052"];
} else if wave == "W6" {
    packets = ["EOS-060", "EOS-061"];
} else if wave == "W7" {
    packets = ["EOS-070", "EOS-071"];
} else if wave == "W8" {
    packets = ["EOS-080", "EOS-081", "EOS-082"];
} else if wave == "W9" {
    packets = ["EOS-090"];
} else {
    pause("verification", "Unknown wave. Use W0 through W9.");
}

let plan_path = "docs/work-in-progress/EXECUTABLE_OPERATOR_SWARM_PLAN_2026-09-19.md";

phase("Mechanical");
let mech_prompt = "";
mech_prompt += "You are the mechanical checker for executable-operator swarm wave ";
mech_prompt += wave;
mech_prompt += ". This is a round-gate, not implementation.\n";
mech_prompt += "Read ";
mech_prompt += plan_path;
mech_prompt += " section 7.\n";
mech_prompt += "Run: powershell -ExecutionPolicy Bypass -File scripts/validate-executable-operator-plan.ps1\n";
mech_prompt += "Run: git status --porcelain and git diff --name-only\n";
mech_prompt += "Off-limits unless the wave brief names them: gguf_bridge/, inference/moe/, ";
mech_prompt += "q42/p64_weight/layout.rs, compiler.rs, inference_agent/decode.rs, sticky_infer.rs, prefill_executor.rs.\n";
mech_prompt += "Set validator_ok true only if the validator exits 0.\n";
mech_prompt += "Set off_limits_clean true only if this wave did not modify off-limits paths.\n";
mech_prompt += "Set wave_packets_review_or_done true only if every packet in ";
mech_prompt += json_encode(packets);
mech_prompt += " is REVIEW or DONE on the live board (not READY/IN_PROGRESS/PLANNED unless the wave is W0 inventory still READY — then set false and explain in notes).\n";
mech_prompt += "Do not edit files. Do not mark packets DONE.\n";

let mechanical = agent(mech_prompt, #{
    label: "eo-mechanical",
    capability_mode: "execute",
    output_schema: mechanical_schema,
});

let mech_ok = mechanical != () && mechanical.success && mechanical.output != ()
    && mechanical.output.validator_ok == true
    && mechanical.output.off_limits_clean == true
    && mechanical.output.wave_packets_review_or_done == true;

if (!mech_ok) {
    let why = "mechanical gate failed";
    if mechanical != () && mechanical.success && mechanical.output != () && mechanical.output.notes != () {
        why = mechanical.output.notes;
    }
    let report = "# EO round-check FAIL\n\nWave: " + wave + "\n\nMechanical: FAIL\n\n" + why + "\n";
    let path = write_scratch_file("eo-round-check.md", report);
    complete(#{ pass: false, wave: wave, reason: "mechanical", path: path });
}

phase("Review");
let jobs = [];
for p in packets {
    let prompt = "";
    prompt += "You are an independent reviewer for swarm packet ";
    prompt += p;
    prompt += " on wave ";
    prompt += wave;
    prompt += ". You did not implement this packet.\n";
    prompt += "Read ";
    prompt += plan_path;
    prompt += " — the packet row, frozen decisions D1-D10, allowed writes for this wave, and section 7.3.\n";
    prompt += "Read the files named in Allowed writes. Use read_file and grep. Do not answer from memory.\n";
    prompt += "Do not edit files. Do not use git worktrees.\n";
    prompt += "Verdict ACCEPT_DONE only if the acceptance column is true in source and tests you ran or inspected.\n";
    prompt += "Verdict REOPEN if implementation exists but acceptance is missed.\n";
    prompt += "Verdict NOT_READY if the packet is not yet implemented or still READY/PLANNED/IN_PROGRESS.\n";
    prompt += "Empty defects is valid only after you inspected the files (or confirmed they do not exist yet for NOT_READY).\n";
    prompt += "packet must equal ";
    prompt += p;
    prompt += ".\n";
    jobs.push(#{
        prompt: prompt,
        label: "eo-review-" + p,
        capability_mode: "read-only",
        output_schema: packet_schema,
    });
}
let reviews = parallel(jobs);

phase("Gate");
let accepted = 0;
let reopen = 0;
let not_ready = 0;
let unverified = 0;
let lines = "# EO round-check\n\nWave: " + wave + "\n\nMechanical: PASS\n\n";
let i = 0;
for r in reviews {
    let id = packets[i];
    if r == () || !r.success || r.output == () {
        unverified += 1;
        lines += "- " + id + ": UNVERIFIED (reviewer failed)\n";
    } else {
        let v = r.output.verdict;
        let ev = r.output.evidence;
        if v == "ACCEPT_DONE" && ev != () && ev != "" {
            accepted += 1;
            lines += "- " + id + ": ACCEPT_DONE — " + ev + "\n";
        } else if v == "REOPEN" {
            reopen += 1;
            lines += "- " + id + ": REOPEN — " + ev + "\n";
        } else if v == "NOT_READY" {
            not_ready += 1;
            lines += "- " + id + ": NOT_READY — " + ev + "\n";
        } else {
            unverified += 1;
            lines += "- " + id + ": UNVERIFIED (bad verdict)\n";
        }
    }
    i += 1;
}

let n = packets.len();
let pass = accepted == n && reopen == 0 && not_ready == 0 && unverified == 0;
let verdict = "FAIL";
if pass { verdict = "PASS"; }
lines += "\nAccepted " + accepted.to_string() + "/" + n.to_string() + "\n";
lines += "Verdict: " + verdict + "\n";
lines += "\nSupervisor still must update the swarm board round-gate log and progress log. This workflow does not self-DONE packets.\n";
log(verdict + " " + wave + " " + accepted.to_string() + "/" + n.to_string());
let path = write_scratch_file("eo-round-check.md", lines);
complete(#{
    pass: pass,
    wave: wave,
    accepted: accepted,
    total: n,
    reopen: reopen,
    not_ready: not_ready,
    unverified: unverified,
    path: path,
});
