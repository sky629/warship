#!/bin/sh
# 계층 사이의 금지된 의존을 검사합니다. 금지된 의존이 하나라도 있으면 실패합니다.
#
# 허용되는 의존은 각 크레이트의 Cargo.toml이 정하고, 이 스크립트는 직접 의존과 간접 의존을 모두 검사합니다.
# 대상 플랫폼에 따라 달라지는 의존도 보도록, 배포하는 플랫폼을 모두 지정해서 의존 목록을 만듭니다.
# `--target all`을 쓰지 않는 이유: 실험용 설정(emscripten의 불안정 cfg)에서만 생기는 의존까지 포함해서 잘못 실패합니다.
#
# 실행: sh scripts/check-deps.sh

set -eu

TARGETS="--target aarch64-apple-darwin --target x86_64-apple-darwin --target x86_64-pc-windows-msvc --target x86_64-unknown-linux-gnu --target wasm32-unknown-unknown"

# 소켓이나 비동기 실행기를 가져오는 크레이트입니다.
SOCKETS="tokio tokio-tungstenite tungstenite ewebsock mio"
APPS="warship_server warship_bot warship_client_core"
ADAPTERS="warship_ws_server warship_ws_client"
ROOTS="warship_dedicated warship_client"

failed=0

# 크레이트 자신과, 그 크레이트가 직접 또는 간접으로 의존하는 패키지의 이름을 한 줄에 하나씩 출력합니다.
# cargo tree는 플랫폼마다 트리를 하나씩 출력하고 사이에 빈 줄을 넣으므로, 빈 줄을 빼고 중복을 없앱니다.
deps_of() {
    # shellcheck disable=SC2086
    cargo tree -p "$1" $TARGETS -e normal,build,dev --prefix none --format '{p}' | awk 'NF { print $1 }' | sort -u
}

# 사용법: forbid <크레이트> <금지된 패키지>...
forbid() {
    crate=$1
    shift
    deps=$(deps_of "$crate")
    for banned in "$@"; do
        if printf '%s\n' "$deps" | grep -qx "$banned"; then
            echo "FAIL: $crate depends on $banned" >&2
            # shellcheck disable=SC2086
            cargo tree -p "$crate" $TARGETS -e normal,build,dev -i "$banned" 2>&1 | head -15 >&2 || true
            failed=1
        fi
    done
}

# shellcheck disable=SC2086 # 목록 변수는 단어 단위로 나누어 넘깁니다.
{
    forbid warship_protocol bevy $SOCKETS $APPS $ADAPTERS $ROOTS
    forbid warship_server bevy $SOCKETS warship_bot warship_client_core $ADAPTERS $ROOTS
    forbid warship_bot bevy $SOCKETS warship_protocol warship_server warship_client_core $ADAPTERS $ROOTS
    forbid warship_client_core bevy $SOCKETS warship_server warship_bot $ADAPTERS $ROOTS
    forbid warship_ws_server bevy ewebsock $APPS warship_ws_client $ROOTS
    forbid warship_ws_client bevy tokio tokio-tungstenite mio $APPS warship_ws_server $ROOTS
    forbid warship_dedicated bevy ewebsock warship_client_core warship_ws_client warship_client
    forbid warship_client warship_server warship_bot tokio tokio-tungstenite warship_ws_server warship_dedicated
}

# 게임 규칙은 외부 크레이트에 전혀 의존하지 않습니다.
rules_deps=$(deps_of warship_rules | grep -vx warship_rules || true)
if [ -n "$rules_deps" ]; then
    echo "FAIL: warship_rules must have no dependencies, found:" >&2
    echo "$rules_deps" >&2
    failed=1
fi

if [ "$failed" -ne 0 ]; then
    exit 1
fi
echo "dependency rules hold"
