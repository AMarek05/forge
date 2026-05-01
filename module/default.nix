# forge Home-manager module
# Thin wrapper — all generation logic lives in languages.nix and includes.nix.

{
  config,
  lib,
  pkgs,
  ...
}@args:

let
  cfg = config.forge;

  compDir = ".local/share/zsh/site-functions";

  # Avoid naming the module argument `forge` to prevent collisions with `config.forge`.
  # Pass your package via `extraSpecialArgs = { forgePkg = ...; }` in your flake setup.
  forge-binary = args.forgePkg or null;

  langs = import ./languages.nix { inherit lib pkgs; };
  incs  = import ./includes.nix  { inherit lib pkgs; };

  zsh-completion = builtins.replaceStrings [ "@JQ@" ] [ "${pkgs.jq}/bin/jq" ] (
    builtins.readFile ./completion.zsh
  );

  # Config file as store symlink
  config-json-file = builtins.toFile "config.json" (
    builtins.toJSON {
      sync_base   = cfg.syncBase;
      editor      = cfg.editor;
      tmux_bin    = cfg.tmuxBinary;
      github_user = cfg.githubUser;
      lang_dir    = "${cfg.configDir}/langs";
      include_dir = "${cfg.configDir}/includes";
    }
  );

in

{
  options.forge = {
    enable = lib.mkEnableOption "forge — tmux sessionizer";

    configDir = lib.mkOption {
      default = "${config.home.homeDirectory}/.forge";
      type = lib.types.path;
      description = "Directory where forge stores config and runtime state";
    };

    syncBase = lib.mkOption {
      default = "${config.home.homeDirectory}/sync";
      type = lib.types.path;
      description = "Root directory where projects live";
    };

    editor = lib.mkOption {
      default = "${pkgs.neovim}/bin/nvim";
      type = lib.types.str;
      description = "Editor binary";
    };

    githubUser = lib.mkOption {
      default = null;
      type = lib.types.nullOr lib.types.str;
      description = "GitHub username for git include";
    };

    tmuxBinary = lib.mkOption {
      default = "${pkgs.tmux}/bin/tmux";
      type = lib.types.str;
      description = "tmux binary path";
    };

    package = lib.mkOption {
      default = forge-binary;
      type = lib.types.nullOr lib.types.package;
      description = "forge binary package";
    };
  };

  config = lib.mkIf cfg.enable {

    home.packages = lib.mkIf (cfg.package != null) [ cfg.package ];

    home.file."${compDir}/_forge".text = zsh-completion;

    programs.zsh = {
      enable = true;
      initContent = lib.mkOrder 550 ''
        fpath=("${config.home.homeDirectory}/${compDir}" $fpath)
      '';
    };

    # config.json as store symlink
    home.file."${cfg.configDir}/config.json".source = config-json-file;

    # langs/default and includes/default — store dirs with language/include files
    home.file."${cfg.configDir}/langs/default".source     = langs.lang-dir;
    home.file."${cfg.configDir}/includes/default".source = incs.include-dir;

    # custom/ dirs — empty placeholders for user-added langs/includes
    # forge sync --langs and --includes scan default/ + custom/ and write the JSONs
    home.file."${cfg.configDir}/langs/custom".source      = pkgs.runCommand "forge-langs-custom" {} "mkdir -p $out";
    home.file."${cfg.configDir}/includes/custom".source   = pkgs.runCommand "forge-includes-custom" {} "mkdir -p $out";

    home.activation = lib.mkOrder 900 ''
      # Pre-populate langs.json and includes.json at activation time
      export FORGE_CONFIG_DIR="${cfg.configDir}"
      ${lib.getExe' cfg.package "forge"} sync --langs
      ${lib.getExe' cfg.package "forge"} sync --includes
    '';

    home.sessionVariables = {
      FORGE_CONFIG_DIR = cfg.configDir;
    };
  };
}