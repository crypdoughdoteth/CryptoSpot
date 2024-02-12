use anyhow::{bail, Result};
use dialoguer::{theme::ColorfulTheme, FuzzySelect, MultiSelect};
use serde::{Deserialize, Serialize};
use tabled::{settings::Style, Table, Tabled};

#[derive(Serialize, Deserialize, Debug)]
struct PriceData {
    data: AssetData,
}

#[derive(Serialize, Deserialize, Debug, Tabled)]
struct AssetData {
    amount: String,
    base: String,
    currency: String,
}

const ASSETS: &[&str] = &[
    "ETH", "BTC", "SUI", "NAVX", "CETUS", "SOL", "SEI", "TIA", "APT", "MATIC", "FTM", "OP", "ARB", "LINK", "DOT",
    "ADA", "AVAX", "LUNA", "ATOM", "ALGO", "XLM", "XRP", "DOGE", "SHIB", "LTC", "BCH", "EOS", "XTZ",
];
const BASE: &[&str] = &["USD", "CAD", "AUD", "INR", "EUR", "GBP", "COP", "JPY", "CNY", "HKD"];

#[tokio::main]
async fn main() -> Result<()> {
    let selection = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Which assets do you want the price of?")
        .items(ASSETS)
        .defaults(vec![false; ASSETS.len()].as_slice())
        .interact()?;

    let base_selection = BASE[FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Which currency should the prices be denominated in?")
        .items(BASE)
        .default(0)
        .interact()?];
    
    if selection.is_empty() {
        bail!("No assets selected");
    }

    let mut handles = Vec::new();

    for i in selection.into_iter() {
        let handle: tokio::task::JoinHandle<anyhow::Result<PriceData>> = tokio::spawn(async move {
            let asset = ASSETS[i];
            let url = format!(
                "https://api.coinbase.com/v2/prices/{}-{}/spot",
                asset, base_selection
            );
            let price: PriceData = reqwest::Client::new()
                .get(url)
                .send()
                .await?
                .json::<PriceData>()
                .await?;
            Ok(price)
        });
        handles.push(handle);
    }

    let mut results = vec![];
    for handle in handles {
        results.push(handle.await??.data);
    }

    let table = Table::new(results).with(Style::modern()).to_string();
    println!("\n{}", table);

    Ok(())
}
