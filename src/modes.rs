use crate::{
    app_config, commands, config, error::Error, previous, previous::Slot, ModeArgs, DEST,
    KUBECONFIG,
};

fn current_namespace_for(config: &kube::config::Kubeconfig, ctx: &str) -> Option<String> {
    config
        .contexts
        .iter()
        .find(|c| c.name == ctx)
        .and_then(|c| c.context.as_ref())
        .and_then(|c| c.namespace.clone())
}

fn resolve(args_value: Option<String>, slot: Slot) -> Result<Option<String>, Error> {
    match args_value.as_deref().map(str::trim) {
        Some("-") => match previous::read(&DEST, slot) {
            Some(v) => Ok(Some(v)),
            None => Err(Error::NoItemSelected { prompt: slot.label() }),
        },
        Some(v) => Ok(Some(v.to_string())),
        None => Ok(None),
    }
}

pub fn default_context(args: ModeArgs) -> Result<(), Error> {
    let config = config::get(None);

    if args.current {
        println!(
            "{}",
            config
                .config
                .current_context
                .as_deref()
                .unwrap_or("No current context set")
        );
        return Ok(());
    }

    let current = config.config.current_context.clone();

    let resolved = resolve(args.value, Slot::GlobalContext)?;
    let ctx = match resolved {
        Some(v) => v,
        None => {
            let options: Vec<String> = config
                .config
                .contexts
                .iter()
                .map(|context| context.name.to_string())
                .collect();

            commands::selectable_list(options, app_config::get())
                .ok_or(Error::NoItemSelected { prompt: "context" })?
        }
    };

    if let Some(target) = config
        .configs
        .iter()
        .find(|(kubeconfig, _)| {
            kubeconfig
                .contexts
                .iter()
                .any(|context| context.name == ctx)
        })
        .map(|(_, path)| path.clone())
    {
        if let Some(prev) = current.as_deref() {
            if prev != ctx {
                previous::write(&DEST, Slot::GlobalContext, prev);
            }
        }
        commands::set_default_context(&ctx, &target);
        // TODO: We should move the target to the front of the line instead of inserting a
        // duplicate
        println!("{}:{}", target.to_string_lossy(), KUBECONFIG.as_str());
    }

    Ok(())
}

pub fn context(args: ModeArgs) -> Result<(), Error> {
    let current_session = config::get_current_session();
    if args.current {
        println!(
            "{}",
            current_session
                .current_context
                .as_deref()
                .unwrap_or("No current context set")
        );
        return Ok(());
    }

    let current = current_session.current_context.clone();

    let config = config::get(None);
    let resolved = resolve(args.value, Slot::SessionContext)?;
    let ctx = match resolved {
        Some(v) => v,
        None => {
            let options: Vec<String> = config
                .config
                .contexts
                .iter()
                .map(|context| context.name.to_string())
                .collect();

            commands::selectable_list(options, app_config::get())
                .ok_or(Error::NoItemSelected { prompt: "context" })?
        }
    };

    if let Some(prev) = current.as_deref() {
        if prev != ctx {
            previous::write(&DEST, Slot::SessionContext, prev);
        }
    }

    let set_context_result =
        commands::set_context(&ctx, &DEST, &current_session).map_err(Error::SetContext);

    if set_context_result.is_ok() {
        println!(
            "{}/{}:{}",
            &DEST.as_str(),
            str::replace(&set_context_result.unwrap(), ":", "_"),
            *KUBECONFIG
        );
    }

    Ok(())
}

pub fn namespace(args: ModeArgs) -> Result<(), Error> {
    let config = config::get_current_session();
    let current_ctx = &config
        .current_context
        .as_deref()
        .unwrap_or("No current context set");
    if args.current {
        if let Some(ctx) = config.contexts.iter().find(|x| {
            x.name
                == config
                    .current_context
                    .as_deref()
                    .unwrap_or("No current context set")
        }) {
            let namespace = ctx
                .context
                .as_ref()
                .and_then(|c| c.namespace.as_deref())
                .unwrap_or("default");

            println!("{}", namespace);
        } else {
            println!("default");
        }
        return Ok(());
    }

    let current_ns = current_namespace_for(&config, current_ctx);

    let resolved = resolve(args.value, Slot::SessionNamespace)?;
    let ns = match resolved {
        Some(v) => v,
        None => {
            let namespaces: Vec<String> = commands::get_namespaces();
            commands::selectable_list(namespaces, app_config::get()).ok_or(
                Error::NoItemSelected {
                    prompt: "namespace",
                },
            )?
        }
    };

    if let Some(prev) = current_ns.as_deref() {
        if prev != ns {
            previous::write(&DEST, Slot::SessionNamespace, prev);
        }
    }

    let result = commands::set_namespace(current_ctx, &ns, &DEST, &config);

    println!(
        "{}/{}:{}",
        &DEST.as_str(),
        str::replace(&result, ":", "_"),
        *KUBECONFIG
    );
    Ok(())
}

pub fn default_namespace(args: ModeArgs) -> Result<(), Error> {
    let current_session = config::get_current_session();
    let config = config::get(None);
    let ctx = &current_session
        .current_context
        .as_deref()
        .unwrap_or("No current context set");

    if args.current {
        if let Some(ctx) = current_session.contexts.iter().find(|x| {
            x.name
                == current_session
                    .current_context
                    .as_deref()
                    .unwrap_or("No current context set")
        }) {
            let namespace = ctx
                .context
                .as_ref()
                .and_then(|c| c.namespace.as_deref())
                .unwrap_or("default");

            println!("{}", namespace);
        } else {
            println!("default");
        }
        return Ok(());
    }

    let current_ns = current_namespace_for(&config.config, ctx);

    let resolved = resolve(args.value, Slot::GlobalNamespace)?;
    let ns = match resolved {
        Some(v) => v,
        None => {
            let namespaces: Vec<String> = commands::get_namespaces();
            commands::selectable_list(namespaces, app_config::get()).ok_or(
                Error::NoItemSelected {
                    prompt: "namespace",
                },
            )?
        }
    };

    if let Some(target) = config
        .configs
        .iter()
        .find(|(kubeconfig, _)| {
            kubeconfig
                .contexts
                .iter()
                .any(|context| context.name == *ctx)
        })
        .map(|(_, path)| path.clone())
    {
        if let Some(prev) = current_ns.as_deref() {
            if prev != ns {
                previous::write(&DEST, Slot::GlobalNamespace, prev);
            }
        }
        commands::set_default_namespace(&ns, ctx, &target);
    }

    let result = commands::set_namespace(ctx, &ns, &DEST, &current_session);
    println!(
        "{}/{}:{}",
        &DEST.as_str(),
        str::replace(&result, ":", "_"),
        *KUBECONFIG
    );

    Ok(())
}

pub fn completion_context(args: ModeArgs) {
    let config = config::get(None);

    let search_value = args.value.as_deref().unwrap_or("");

    let options: Vec<String> = config
        .config
        .contexts
        .iter()
        .filter(|context| context.name.starts_with(search_value))
        .map(|context| context.name.clone())
        .collect();

    println!("{}", options.join(" "));
}

pub fn completion_namespace(args: ModeArgs) {
    let namespaces = commands::get_namespaces();
    let mut options = Vec::new();

    let search_value = args.value.as_deref().unwrap_or("");

    for ns in &namespaces {
        if ns.starts_with(search_value) {
            options.push(ns.to_string());
        }
    }

    println!("{}", options.join(" "));
}

