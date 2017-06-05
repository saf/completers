COMPLETERS_DIR=$(dirname $(readlink -f ${BASH_SOURCE}))

function completers_complete {
    read point line < <("${COMPLETERS_DIR}/../target/debug/completers" --point="${READLINE_POINT}" "${READLINE_LINE}")
    READLINE_LINE=$line
    READLINE_POINT=$point
}

bind -x '"`":"completers_complete"'
