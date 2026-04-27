#compdef forge

forge() {
  local -a commands
  commands=(
    "create:Create a new project"
    "remove:Remove a project from the index"
    "list:List all projects"
    "sync:Re-scan FORGE_SYNC_BASE and rebuild the index"
    "cd:Print project path to stdout"
    "session:Switch to or create a tmux session"
    "pick:Interactive fzf session picker"
    "setup:Run setup scripts for a project"
    "include:List or show include modules"
    "lang:List or add language packs"
    "overseer:Run or manage overseer.nvim task templates"
    "overseer-def:Print JSON overseer task definition"
    "edit:Edit project's .wl in $EDITOR"
    "open:Open project directory in $EDITOR"
  )

  if [[ "${words[CURRENT]}" == -* ]]; then
    local -a options=(
      "--help:Show help"
      "--version:Show version"
    )
    _describe "options" options
    return
  fi

  if (( CURRENT == 2 )); then
    _describe "commands" commands
    return
  fi

  local cmd="${words[2]}"
  local -a flags

  case "$cmd" in
    create)
      flags=(
        "--lang:Language (required)"
        "--no-open:Skip opening .wl in $EDITOR"
        "--setup:Run setup scripts after creating .wl"
        "--include:Pre-populate includes field (comma-separated)"
        "--path:Override project path"
        "--run:Run arbitrary shell command after creation"
        "--editor:Open $EDITOR after full creation"
        "--dry-run:Print actions without executing"
      )
      ;;
    remove|list|cd|edit|open|overseer-def)
      local -a projects
      projects=($( @JQ@ -r '.projects[].name' ~/.forge-index.json 2>/dev/null))
      if (( ''${#projects} )); then
        _describe "projects" projects
      else
        _message "no projects found — run forge sync first"
      fi
      ;;
    session)
      flags=(
        "--setup:Run setup scripts in the session"
        "--open:Open project in $EDITOR after switching"
      )
      ;;
    pick)
      flags=(
        "--tags:Filter by tags (comma-separated)"
      )
      ;;
    setup)
      flags=(
        "--dry-run:Print actions without executing"
      )
      ;;
    include)
      flags=(
        "--list:List all available includes"
      )
      ;;
    lang)
      flags=(
        "--list:List all available languages"
        "--add:Add a new language pack"
      )
      ;;
    overseer)
      flags=(
        "--regen:Regenerate all project templates"
        "--rm:Remove project's templates"
        "--setup:Run setup scripts for overseer include"
      )
      ;;
  esac

  if [[ ''${#flags[@]} -gt 0 ]]; then
    _describe "flags" flags
  fi
}

forge "$@"
