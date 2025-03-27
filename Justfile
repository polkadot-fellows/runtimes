coverage ahm_dry_run_repo="../ahm-dryrun":
    #!/usr/bin/env bash

    runtimes=$(pwd)
    cd {{ahm_dry_run_repo}}
    just coverage update $runtimes
