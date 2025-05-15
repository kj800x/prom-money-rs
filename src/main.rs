use metrics::{describe_gauge, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{env, thread};

#[derive(Debug, Serialize, Deserialize)]
struct Account {
    id: String,
    name: String,
    r#type: String,
    balance: i64,
    cleared_balance: i64,
    direct_import_linked: bool,
    direct_import_in_error: bool,
    deleted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    data: Data,
}

#[derive(Debug, Serialize, Deserialize)]
struct Data {
    accounts: Vec<Account>,
    server_knowledge: i64,
}

fn main() {
    tracing_subscriber::fmt::init();

    let builder = PrometheusBuilder::new();
    builder
        // If an account disappears for 3 hours, it's probably gone.
        .idle_timeout(MetricKindMask::ALL, Some(Duration::from_secs(60 * 60 * 3)))
        .with_http_listener(([0, 0, 0, 0], 9090))
        .install()
        .expect("Failed to install Prometheus recorder");

    describe_gauge!(
        "ynab_account_balance",
        "The current balance of the account in YNAB in dollars."
    );

    let client = reqwest::blocking::Client::new();

    loop {
        println!("Hi Suruu...");
        println!("Fetching accounts...");
        let response: Response = client
            .get(format!(
                "https://api.ynab.com/v1/budgets/{}/accounts",
                env::var("YNAB_BUDGET_ID")
                    .expect("YNAB_BUDGET_ID environment variable must be set")
            ))
            .bearer_auth(
                env::var("YNAB_API_KEY").expect("YNAB_API_KEY environment variable must be set"),
            )
            .header("accept", "application/json")
            .send()
            .unwrap()
            .json()
            .unwrap();

        response.data.accounts.iter().filter(|account| { account.direct_import_linked }).for_each(|account| {
            gauge!("ynab_account_balance", account.balance as f64 / 1000.0, "account_type" => account.r#type.clone(), "account_id" => account.id.clone(), "account_name" => account.name.clone());
        });

        // Sleep 1 hour.
        println!("Sleeping...");
        thread::sleep(Duration::from_secs(60 * 60));
    }
}
