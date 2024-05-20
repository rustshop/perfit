{ config, lib, pkgs, ... }:
with lib;

let
  eachPerfitd = filterAttrs (perfitdName: cfg: cfg.enable) config.services.perfitd;
  perfitdOpts = { config, lib, name, ... }: {

    options = {

      enable = mkEnableOption (lib.mdDoc "perfit");

      user = mkOption {
        type = types.str;
        default = "perfitd-${name}";
        description = "The user as which to run perfitd.";
      };

      group = mkOption {
        type = types.str;
        default = config.user;
        description = "The group as which to run perfitd.";
      };

      extraEnvironment = mkOption {
        type = types.attrsOf types.str;
        description = lib.mdDoc "Extra Environment variables to pass to the perfitd.";
        default = {
          RUST_BACKTRACE = "1";
        };
        example = {
          RUST_LOG = "info";
          RUST_BACKTRACE = "1";
        };
      };

      package = mkOption {
        type = types.nullOr types.package;
        default = pkgs.perfitd or null;
        defaultText = lib.literalExpression "pkgs.perfit (after available)";
        description = lib.mdDoc ''
          Package of the perfitd to use.
        '';
      };

      openFirewall = mkOption {
        type = types.bool;
        default = false;
        description = lib.mdDoc "Opens port in firewall for perfitd's p2p port";
      };
      port = mkOption {
        type = types.port;
        default = 5050;
        description = lib.mdDoc "Port to bind on for p2p connections from peers";
      };
      bind = mkOption {
        type = types.str;
        default = "[::1]";
        description = lib.mdDoc "Address to bind on for p2p connections from peers";
      };

      dataDir = mkOption {
        type = types.str;
        default = "/var/lib/perfitd/";
        readOnly = true;
        description = lib.mdDoc ''
          Path to the data dir perfitd will use to store its data.
          Note that due to using the DynamicUser feature of systemd, this value should not be changed
          and is set to be read only.
        '';
      };

      rootAccessTokenFile = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = lib.mdDoc ''
          If set PERFITD_ROOT_ACCESS_TOKEN will be set to the value in that file.
        '';
      };
    };
  };
in
{
  options = {
    services.perfitd = mkOption {
      type = types.attrsOf (types.submodule perfitdOpts);
      default = { };
      description = lib.mdDoc "Specification of one or more perfitd instances.";
    };
  };

  config = mkIf (eachPerfitd != { }) {

    assertions = flatten
      (mapAttrsToList
        (perfitdName: cfg: [
          {
            assertion = cfg.package != null;
            message = ''
              `services.perfitd.${perfitdName}.package` must be set manually until `perfitd` is available in nixpkgs.
            '';
          }
        ])
        eachPerfitd);

    networking.firewall.allowedTCPPorts = flatten
      (mapAttrsToList
        (perfitdName: cfg:
          (
            if cfg.openFirewall then [
              cfg.port
            ] else [ ]
          )
        )
        eachPerfitd);


    systemd.services =
      mapAttrs'
        (perfitdName: cfg: (
          nameValuePair "perfitd-${perfitdName}" (
            let
              startScript = pkgs.writeShellScript "perfitd-start" (
                ''
                  set -euo pipefail
                '' +
                (if cfg.rootAccessTokenFile != null then
                  ''
                    secret=$(${pkgs.coreutils}/bin/head -n 1 "${cfg.rootAccessTokenFile}")
                    PERFITD_ROOT_ACCESS_TOKEN="''${secret}"
                  ''
                else
                  "") +
                ''
                  exec ${cfg.package}/bin/perfitd
                ''
              );
            in
            {
              description = "Perfit Server";
              documentation = [ "https://github.com/perfit/perfit/" ];
              wantedBy = [ "multi-user.target" ];
              environment = lib.mkMerge ([
                {
                  PERFITD_LISTEN = "${cfg.bind}:${builtins.toString cfg.port}";
                  PERFITD_DB_PATH = "${cfg.dataDir}/perfitd.db";
                }
                cfg.extraEnvironment
              ]);
              serviceConfig = {
                User = cfg.user;
                Group = cfg.group;

                Restart = "always";
                RestartSec = 10;
                StartLimitBurst = 5;
                UMask = "077";
                LimitNOFILE = "100000";

                LockPersonality = true;
                ProtectClock = true;
                ProtectControlGroups = true;
                ProtectHostname = true;
                ProtectKernelLogs = true;
                ProtectKernelModules = true;
                ProtectKernelTunables = true;
                PrivateMounts = true;
                RestrictAddressFamilies = [ "AF_INET" "AF_INET6" ];
                RestrictNamespaces = true;
                RestrictRealtime = true;
                SystemCallArchitectures = "native";
                SystemCallFilter = [
                  "@system-service"
                  "~@privileged"
                ];
                StateDirectory = "perfitd";
                StateDirectoryMode = "0700";
                ExecStart = startScript;


                # Hardening measures
                PrivateTmp = "true";
                ProtectSystem = "full";
                NoNewPrivileges = "true";
                PrivateDevices = "true";
                MemoryDenyWriteExecute = "true";
              };
            }
          )
        ))
        eachPerfitd;


    users.users = mapAttrs'
      (perfitdName: cfg: (
        nameValuePair "perfitd-${perfitdName}" {
          name = cfg.user;
          group = cfg.group;
          description = "Perfit daemon user";
          home = cfg.dataDir;
          isSystemUser = true;
        }
      ))
      eachPerfitd;

    users.groups = mapAttrs'
      (perfitdName: cfg: (
        nameValuePair "${cfg.group}" { }
      ))
      eachPerfitd;
  };
}
