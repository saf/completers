COMPLETERS_DIR=$(dirname $(readlink -f ${BASH_SOURCE}))

target=${1:-debug}

function completers_complete {
    read point line < <("${COMPLETERS_DIR}/../target/$target/completers" --point="${READLINE_POINT}" "${READLINE_LINE}")
    READLINE_LINE=$line
    READLINE_POINT=$point
}

bind -x '"`":"completers_complete"'
