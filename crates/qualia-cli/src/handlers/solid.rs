use std::path::PathBuf;

use crate::cli::SolidAction;

pub async fn handle(action: SolidAction) {
    match action {
        SolidAction::Serve {
            host,
            port,
            data_root,
            public_base,
            demo_oidc,
            no_demo_oidc,
        } => {
            let data_root = data_root.unwrap_or_else(|| {
                std::env::var("QUALIA_SOLID_POD_ROOT")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| std::env::temp_dir().join("qualia-solid-pod"))
            });
            let public_base =
                public_base.unwrap_or_else(|| format!("http://{host}:{port}"));
            let cfg = qualia_solid_bridge::BridgeConfig {
                listen: format!("{host}:{port}").parse().expect("invalid host:port"),
                data_root,
                public_base,
                demo_oidc: demo_oidc && !no_demo_oidc,
            };
            qualia_solid_bridge::run_bridge(cfg).await;
        }
        SolidAction::Fetch { url, token, out } => {
            match qualia_solid_bridge::fetch_resource(&url, token.as_deref()).await {
                Ok(r) => {
                    println!("status        : {}", r.status);
                    println!("content-type  : {}", r.content_type);
                    println!("quin_count    : {}", r.quin_count);
                    println!("url           : {}", r.url);
                    if let Some(path) = out {
                        if let Err(e) = std::fs::write(&path, r.body.as_bytes()) {
                            eprintln!("write {}: {e}", path.display());
                        } else {
                            println!("wrote         : {}", path.display());
                        }
                    } else {
                        let preview: String = r.body.chars().take(800).collect();
                        println!("--- body (preview) ---\n{preview}");
                    }
                }
                Err(e) => eprintln!("solid fetch failed: {e}"),
            }
        }
        SolidAction::Put {
            url,
            file,
            content_type,
            token,
        } => match std::fs::read(&file) {
            Ok(body) => {
                match qualia_solid_bridge::put_resource(
                    &url,
                    &body,
                    &content_type,
                    token.as_deref(),
                )
                .await
                {
                    Ok(status) => println!("PUT ok status={status} url={url}"),
                    Err(e) => eprintln!("solid put failed: {e}"),
                }
            }
            Err(e) => eprintln!("read {}: {e}", file.display()),
        },
        SolidAction::Post {
            container,
            file,
            content_type,
            slug,
            token,
        } => match std::fs::read(&file) {
            Ok(body) => {
                match qualia_solid_bridge::post_to_container(
                    &container,
                    &body,
                    &content_type,
                    slug.as_deref(),
                    token.as_deref(),
                )
                .await
                {
                    Ok((status, loc)) => {
                        println!("POST ok status={status}");
                        if let Some(l) = loc {
                            println!("location={l}");
                        }
                    }
                    Err(e) => eprintln!("solid post failed: {e}"),
                }
            }
            Err(e) => eprintln!("read {}: {e}", file.display()),
        },
    }
}
