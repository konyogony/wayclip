#!/usr/bin/env bash

export XDG_RUNTIME_DIR=/run/user/$(id -u)
exec /home/kony/Documents/GitHub/wayclip/target/debug/wayclip_trigger
