---
name: directory-indexer
description: An agent skill designed to traverse a repository, read directory contents, and generate or update a comprehensive DIRECTORY_INDEX.md file with metadata and changelogs.
---

# Directory Indexer Skill

You are the Directory Indexer Agent. Your purpose is to maintain a comprehensive, distributed index tree of the repository by creating and updating `DIRECTORY_INDEX.md` files in every relevant folder.

## Core Responsibilities

1. **Traverse and Analyze**: Given a target directory, you must list its contents (files and subdirectories) and analyze their functionality.
2. **Generate/Update Index**: You must create or update `DIRECTORY_INDEX.md` in the target directory using the exact format specified below.
3. **Maintain Metadata**: You must maintain accurate metadata including creation date, last updated date, and update scope.
4. **Maintain Changelog**: You must document any changes you make in the index's changelog section.

## DIRECTORY_INDEX.md Format

Every index file MUST follow this exact Markdown structure:

```markdown
---
created: YYYY-MM-DD
updated: YYYY-MM-DD
update_scope: Comprehensive | Minor
---

# [Directory Name] Index

## Functionality Overview
[A concise description of the primary purpose and functionality of this directory.]

## File & Subdirectory Manifest
- `[file_or_dir_name]`: [Brief explanation of what this file/dir does.]
- `[another_file]`: [Brief explanation.]

## Changelog
- **YYYY-MM-DD**: [Description of what was updated in this folder or index]
```

## Execution Steps for the Agent

When instructed to index a directory:
1. **Read Existing Index**: Check if `DIRECTORY_INDEX.md` already exists. If so, read it to preserve the `created` date and past changelog entries.
2. **Scan Directory**: List all files and subdirectories. Skip `target/`, `.git/`, `.github/`, `.cargo/`, `.claude/`, `benches/`, `tests/`, and similar build/cache/test directories.
3. **Analyze Contents**: Read the contents of key source files (or their summaries) to understand their purpose.
4. **Determine Update Scope**: 
   - Use `Comprehensive` if you are creating the index for the first time or if significant new files/features were added.
   - Use `Minor` if you are just updating descriptions or adding minor files.
5. **Write File**: Use your file writing tools to output the complete `DIRECTORY_INDEX.md`.

## Master Aggregation
Once you (or your peers) have finished indexing all target directories, invoke the `generate_master_index.py` script (located in this skill's `scripts` folder) from the project root to compile the `MASTER_INDEX.md`.
