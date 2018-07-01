completers_dir_=$(dirname $(readlink -f ${BASH_SOURCE}))

completers_target_=${1:-release}
shift
completers_args_=("$@")

function completers_complete_ {
    read point line < <("${completers_dir_}/../target/$completers_target_/completers" \
			    --point="${READLINE_POINT}" \
			    "${READLINE_LINE}" \
			    "${completers_args_[@]}")
    READLINE_LINE=$line
    READLINE_POINT=$point
}

bind -x '"`":"completers_complete_"'
