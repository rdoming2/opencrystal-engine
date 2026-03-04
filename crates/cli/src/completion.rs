pub(crate) fn run_completion(args: Vec<String>) {
    match args.first().map(String::as_str) {
        Some("bash") => print!("{}", bash_script()),
        Some("zsh") => print!("{}", zsh_script()),
        _ => eprintln!("Usage: cryst completion <bash|zsh>"),
    }
}

fn bash_script() -> &'static str {
    r#"_cryst() {
    local cur prev cmd sub opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev=""
    if (( COMP_CWORD > 0 )); then
        prev="${COMP_WORDS[COMP_CWORD-1]}"
    fi

    case "$prev" in
        --content|--content-dir|--path)
            compopt -o filenames 2>/dev/null
            COMPREPLY=( $(compgen -f -- "$cur") )
            return 0
            ;;
        --render)
            COMPREPLY=( $(compgen -W "auto wide modern" -- "$cur") )
            return 0
            ;;
    esac

    case "$cur" in
        --content=*|--content-dir=*|--path=*)
            local prefix value match
            prefix="${cur%%=*}="
            value="${cur#*=}"
            compopt -o filenames 2>/dev/null
            while IFS= read -r match; do
                COMPREPLY+=("${prefix}${match}")
            done < <(compgen -f -- "$value")
            return 0
            ;;
        --render=*)
            local prefix value
            prefix="${cur%%=*}="
            value="${cur#*=}"
            COMPREPLY=( $(compgen -W "auto wide modern" -- "$value") )
            COMPREPLY=( "${COMPREPLY[@]/#/${prefix}}" )
            return 0
            ;;
    esac

    if (( COMP_CWORD == 1 )); then
        COMPREPLY=( $(compgen -W "play validate new-project build completion" -- "$cur") )
        return 0
    fi

    cmd="${COMP_WORDS[1]}"
    case "$cmd" in
        play)
            opts="--render --content --content-dir"
            ;;
        validate)
            opts="--content --content-dir"
            ;;
        new-project)
            opts="--path"
            ;;
        completion)
            if (( COMP_CWORD == 2 )); then
                COMPREPLY=( $(compgen -W "bash zsh" -- "$cur") )
            fi
            return 0
            ;;
        build)
            if (( COMP_CWORD == 2 )); then
                COMPREPLY=( $(compgen -W "new map upgrade strings new-project docs" -- "$cur") )
                return 0
            fi
            sub="${COMP_WORDS[2]}"
            case "$sub" in
                new)
                    if (( COMP_CWORD == 3 )); then
                        COMPREPLY=( $(compgen -W "spell ability item equipment enemy vehicle shop npc encounter job" -- "$cur") )
                        return 0
                    fi
                    opts="--content --content-dir --name --force"
                    ;;
                map)
                    opts="--content"
                    ;;
                upgrade)
                    opts="--content --content-dir --dry-run"
                    ;;
                strings)
                    opts="--content --content-dir --force"
                    ;;
                new-project)
                    opts="--path"
                    ;;
                docs)
                    opts="-s --schemas -a --architecture -c --content-authoring -j --jobs"
                    ;;
                *)
                    opts="new map upgrade strings new-project docs"
                    ;;
            esac
            ;;
        *)
            return 0
            ;;
    esac

    if [[ "$cur" == -* ]]; then
        COMPREPLY=( $(compgen -W "$opts" -- "$cur") )
    fi
}

complete -F _cryst cryst
"#
}

fn zsh_script() -> &'static str {
    r#"#compdef cryst

_cryst() {
    local cur prev cmd sub
    cur="${words[CURRENT]}"
    prev=""
    if (( CURRENT > 1 )); then
        prev="${words[CURRENT-1]}"
    fi

    case "$prev" in
        --content|--content-dir|--path)
            _files
            return 0
            ;;
        --render)
            compadd -- auto wide modern
            return 0
            ;;
    esac

    if (( CURRENT == 2 )); then
        compadd -- play validate new-project build completion
        return 0
    fi

    cmd="${words[2]}"
    case "$cmd" in
        play)
            if [[ "$cur" == -* ]]; then
                compadd -- --render --content --content-dir
            fi
            return 0
            ;;
        validate)
            if [[ "$cur" == -* ]]; then
                compadd -- --content --content-dir
            fi
            return 0
            ;;
        new-project)
            if [[ "$cur" == -* ]]; then
                compadd -- --path
            fi
            return 0
            ;;
        completion)
            if (( CURRENT == 3 )); then
                compadd -- bash zsh
            fi
            return 0
            ;;
        build)
            if (( CURRENT == 3 )); then
                compadd -- new map upgrade strings new-project docs
                return 0
            fi
            sub="${words[3]}"
            case "$sub" in
                new)
                    if (( CURRENT == 4 )); then
                        compadd -- spell ability item equipment enemy vehicle shop npc encounter job
                        return 0
                    fi
                    if [[ "$cur" == -* ]]; then
                        compadd -- --content --content-dir --name --force
                    fi
                    ;;
                map)
                    if [[ "$cur" == -* ]]; then
                        compadd -- --content
                    fi
                    ;;
                upgrade)
                    if [[ "$cur" == -* ]]; then
                        compadd -- --content --content-dir --dry-run
                    fi
                    ;;
                strings)
                    if [[ "$cur" == -* ]]; then
                        compadd -- --content --content-dir --force
                    fi
                    ;;
                new-project)
                    if [[ "$cur" == -* ]]; then
                        compadd -- --path
                    fi
                    ;;
                docs)
                    if [[ "$cur" == -* ]]; then
                        compadd -- -s --schemas -a --architecture -c --content-authoring -j --jobs
                    fi
                    ;;
                *)
                    compadd -- new map upgrade strings new-project docs
                    ;;
            esac
            return 0
            ;;
    esac
}

compdef _cryst cryst
"#
}
