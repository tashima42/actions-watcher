use serde::{Deserialize, Serialize};
use std::{env, time::Duration};
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};
use reqwest::{header, Error, Client};

#[derive(Deserialize, Debug)]
struct GHAction {
    status: String,
    conclusion: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct ECMNotify {
    step_name: String,
    step_status: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    let run_id = args.get(1).unwrap().clone();
    let name = args.get(2).unwrap().clone();
    let token = std::env::var("GH_TOKEN").unwrap();
    println!("checking {run_id} - {name}");
    scheduler(run_id, name, token).await.unwrap();
    Ok(())
}

async fn scheduler(run_id: String, name: String, token: String) -> Result<(), JobSchedulerError> {
    let sched = JobScheduler::new().await?;

    sched.add(Job::new_async("1/60 * * * * *", move |_uuid, _l| {
        let r = run_id.clone();
        let n = name.clone();
        let t = token.clone();
        Box::pin(async move {
            let action = get_action(r.clone(), t).await.unwrap();

            if action.status == "completed" {
                let ecm_notify = ECMNotify{step_name: n.to_owned(), step_status: action.conclusion.unwrap()};
                slack_notify(ecm_notify).await.unwrap()
            }
        })
    })?
    ).await?;
   
    sched.start().await?;

    tokio::time::sleep(Duration::from_secs(3600)).await;

    Ok(())
}

async fn get_action(run_id: String, token: String) -> Result<GHAction, Error> {
    let gh_base_url = "https://api.github.com".to_string();
    let owner = "rancher".to_string();
    let repo = "rancher".to_string();

    let mut headers = header::HeaderMap::new();
    headers.insert("User-Agent", "Actions-Reader/0.0.1 beta testing".parse().unwrap());
    headers.insert("Accept", "application/vnd.github+json".parse().unwrap());
    headers.insert("Authorization", format!("token {token}").parse().unwrap());

    let url = format!("{gh_base_url}/repos/{owner}/{repo}/actions/runs/{run_id}");
    println!("{url}");

    let client = reqwest::Client::new();
    Ok(client.get(&url).headers(headers).send().await?.json().await?)
}

async fn slack_notify(notification: ECMNotify) -> Result<(), Error> {
    let url = std::env::var("SLACK_WEBHOOK").unwrap();
    Client::new()
        .post(url)
    .json(&notification)
    .send().await?;

    Ok(())
}
