use kira_config::DiscordConfig;
#[cfg(feature = "server")]
use kira_config::ServerConfig;
use tokio::sync::watch;

async fn wait_flag(mut rx: watch::Receiver<bool>) {
    loop {
        if *rx.borrow_and_update() {
            return;
        }

        if rx.changed().await.is_err() {
            return;
        }
    }
}

#[cfg(feature = "server")]
pub async fn run_all(discord: DiscordConfig, server: ServerConfig) -> kira_core::Result<()> {
    let (tx, rx) = watch::channel(false);

    let bot = kira_bot::run(discord, wait_flag(rx.clone()));
    let http = kira_server::serve(&server, wait_flag(rx.clone()));
    tokio::pin!(bot);
    tokio::pin!(http);

    let mut bot_result = None;
    let mut http_result = None;

    tokio::select! {
        result = &mut bot => {
            bot_result = Some(result);
        }
        result = &mut http => {
            http_result = Some(result);
        }
        signal = kira_core::shutdown::wait_for_signal() => {
            signal?;
        }
    }

    let _ = tx.send(true);

    let bot_result = match bot_result {
        Some(result) => result,
        None => bot.await,
    };
    let http_result = match http_result {
        Some(result) => result,
        None => http.await,
    };

    bot_result?;
    http_result?;

    Ok(())
}

#[cfg(not(feature = "server"))]
pub async fn run_bot(discord: DiscordConfig) -> kira_core::Result<()> {
    let (tx, rx) = watch::channel(false);

    let bot = kira_bot::run(discord, wait_flag(rx.clone()));
    tokio::pin!(bot);

    let mut bot_result = None;

    tokio::select! {
        result = &mut bot => {
            bot_result = Some(result);
        }
        signal = kira_core::shutdown::wait_for_signal() => {
            signal?;
        }
    }

    let _ = tx.send(true);

    match bot_result {
        Some(result) => result,
        None => bot.await,
    }
}
