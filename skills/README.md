# infradrift Skills

This directory contains AI agent skill definitions for infradrift, enabling AI coding assistants (Claude Code, OpenAI Codex, etc.) to use infradrift effectively when performing infrastructure drift detection tasks.

## What are skills?

Skills are structured descriptions that teach AI agents how to use a CLI tool. They include:

- **SKILL.md** — The main skill definition: commands, options, patterns, and workflows
- **TRUST.md** — Security and trust information about the tool
- **agents/** — Agent-specific configuration files (e.g., OpenAI Codex)

## Available skills

### `infradrift`

Infrastructure drift detection skill for Terraform/OpenTofu. Teaches agents to:

- Run `infradrift scan` to detect drift against live infrastructure
- Run `infradrift parse` to analyze existing plan files
- Configure `infradrift.toml` ignore rules to suppress expected drift
- Use CLI filters (`--type`, `--name`, `--attr`) to narrow findings
- Choose the right output format for the context (`human`, `json`, `csv`, `table`, `hcl`)
- Interpret exit codes for CI/CD gating

## Enabling the skill

### Claude Code

Add the skill to your Claude Code settings (`.claude/settings.json` or project-level):

```json
{
  "skills": ["./skills/infradrift"]
}
```

Or reference it from a remote source if published:

```json
{
  "skills": ["eukarya-inc/infradrift//skills/infradrift"]
}
```

### OpenAI Codex

The `agents/openai.yaml` file provides the interface definition. Configure it according to your Codex agent setup.

## Directory structure

```
skills/
  README.md              # This file
  infradrift/
    SKILL.md             # Main skill definition
    TRUST.md             # Security and trust information
    agents/
      openai.yaml        # OpenAI Codex agent configuration
```
