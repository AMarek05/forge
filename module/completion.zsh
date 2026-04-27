#compdef forge

_forge() {
  local context state state_descr line
  typeset -A opt_args

  # Define common arguments shared across all subcommands
  # '(-h --help)' ensures that if they type -h, it won't suggest --help afterwards
  local -a common_args=(
    '(-h --help)'{-h,--help}'[Show help for this command]'
  )

  _arguments -C \
    '(- 1 *)'{-h,--help}'[Show help]' \
    '(- 1 *)'{-v,--version}'[Show version]' \
    '1: :->cmds' \
    '*:: :->args'

  case $state in
    cmds)
      local -a commands=(
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
        "edit:Edit project's .wl in \$EDITOR"
        "open:Open project directory in \$EDITOR"
      )
      _describe -t commands "forge commands" commands
      ;;

    args)
      case $words[1] in
        create)
          _arguments $common_args \
            '--lang=[Language (required)]:language' \
            '--no-open[Skip opening .wl in $EDITOR]' \
            '--setup[Run setup scripts after creating .wl]' \
            '--include=[Pre-populate includes field (comma-separated)]:includes' \
            '--path=[Override project path]:_dirs' \
            '--run=[Run arbitrary shell command after creation]:command' \
            '--editor=[Open $EDITOR after full creation]' \
            '--dry-run[Print actions without executing]'
          ;;
        session)
          _arguments $common_args \
            '--setup[Run setup scripts in the session]' \
            '--open[Open project in $EDITOR after switching]'
          ;;
        pick)
          _arguments $common_args \
            '--tags=[Filter by tags (comma-separated)]:tags'
          ;;
        setup)
          _arguments $common_args \
            '--dry-run[Print actions without executing]'
          ;;
        include)
          _arguments $common_args \
            '--list[List all available includes]'
          ;;
        lang)
          _arguments $common_args \
            '--list[List all available languages]' \
            '--add[Add a new language pack]'
          ;;
        overseer)
          _arguments $common_args \
            '--regen[Regenerate all project templates]' \
            '--rm[Remove project templates]' \
            '--setup[Run setup scripts for overseer include]'
          ;;
        remove|cd|edit|open)
          # These take a project name AND the common help flag
          _arguments $common_args \
            '1:project name:'
          ;;
        list|sync|overseer-def)
          # These take NO unique arguments, but still accept the help flag
          _arguments $common_args
          ;;
      esac
      ;;
  esac
}

_forge "$@"
