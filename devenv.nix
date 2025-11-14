{ pkgs, lib, config, inputs, ... }:

{
  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    channel = "stable";
  };

  scripts.mcumgr.exec = ''
    cargo run --release --bin smp-tool -- -t udp $@
  '';

  scripts.test_deploy.exec = ''
    cargo test -p smp-tool --test deployment -- --ignored --nocapture
  '';

  scripts.test_rollback.exec = ''
    cargo test -p smp-tool --test rollback -- --ignored --nocapture
  '';
}
