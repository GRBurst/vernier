{pkgs, ...}: {
  # Pinned environment: every gate runs in here with zero host tools (B8).
  # `devenv shell` creates devenv.lock on first run; commit it.

  env.RUST_BACKTRACE = "1";

  languages.rust.enable = true; # rustc, cargo, clippy, rustfmt

  packages = with pkgs; [
    git
    just
    python3 # tools/spec-check (stdlib only)
    # For spec 001 M3b, only if the udpipe route wins the parser spike:
    # clang / stdenv.cc and pkg-config
  ];

  git-hooks.hooks = {
    verify = {
      enable = true;
      name = "just verify";
      entry = "just verify";
      pass_filenames = false;
      stages = ["pre-commit"];
    };
    commit-tier = {
      enable = true;
      name = "commit names its ceremony tier";
      entry = "${pkgs.writeShellScript "commit-tier" ''
        grep -qE '\((full-spec|spec-delta|direct-patch)\)' "$1" || {
          echo "commit message must name its tier: (full-spec|spec-delta|direct-patch)" >&2
          exit 1
        }
      ''}";
      stages = ["commit-msg"];
    };
  };

  enterShell = ''
    echo "vernier dev shell — run: just verify"
  '';
}
