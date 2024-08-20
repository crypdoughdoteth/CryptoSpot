use anyhow::{bail, Result};
use dialoguer::{theme::ColorfulTheme, FuzzySelect, MultiSelect};
use serde::{Deserialize, Serialize};
use tabled::settings::object::Rows;
use tabled::settings::{themes::Colorization, Color};
use tabled::{settings::Style, Table, Tabled};

#[derive(Debug, Clone, Tabled)]
pub struct CoinbasePriceData {
    current_price: f64,
    daily_percent: f64,
    base: String,
    currency: String,
}

const ASSETS: &[&str] = &[
    "ETH", "BTC", "SUI", "SOL", "DOT", "SEI", "TIA", "APT", "MATIC", "FTM", "OP", "ARB", "LINK",
    "ADA", "HBAR", "AVAX", "ATOM", "ALGO", "XLM", "XRP", "DOGE", "SHIB", "LTC", "BCH", "EOS",
    "XTZ",
];
const BASE: &[&str] = &[
    "USD", "CAD", "AUD", "INR", "EUR", "GBP", "COP", "JPY", "CNY", "HKD",
];

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

    let selection_size = selection.len();
    let mut handles = Vec::with_capacity(selection_size);
    for i in selection.into_iter() {
        let handle: tokio::task::JoinHandle<anyhow::Result<(CoinbasePriceData, Color)>> =
            tokio::spawn(async move {
                let asset = ASSETS[i];
                let url = format!(
                    "https://api.coinbase.com/v2/prices/{}-{}/historic?period=day",
                    asset, base_selection
                );

                let prices: HistoricalPriceData = reqwest::Client::new()
                    .get(url)
                    .send()
                    .await?
                    .json::<HistoricalPriceData>()
                    .await?;

                let recent = prices
                    .data
                    .prices
                    .first()
                    .unwrap()
                    .price
                    .parse::<f64>()
                    .unwrap();

                let day_ago = prices
                    .data
                    .prices
                    .last()
                    .unwrap()
                    .price
                    .parse::<f64>()
                    .unwrap();

                let percent_diff = ((recent - day_ago).abs() / ((recent + day_ago) / 2.0)) * 100.0;

                let res = CoinbasePriceData {
                    current_price: recent,
                    daily_percent: percent_diff,
                    base: base_selection.to_string(),
                    currency: asset.to_string(),
                };

                let daily: Color = match (recent, day_ago) {
                    (r, d) if r > d => Color::BG_GREEN,
                    (r, d) if r < d => Color::BG_RED,
                    _ => Color::BG_WHITE,
                };

                Ok((res, daily))
            });
        handles.push(handle);
    }

    let mut results = Vec::with_capacity(selection_size);
    let mut colors = Vec::with_capacity(selection_size);
    for handle in handles {
        let res = handle.await??;
        results.push(res.0);
        colors.push(res.1);
    }

    let table = Table::new(results)
        .with(Style::modern())
        .with(Colorization::exact(colors, Rows::new(1..)))
        .to_string();

    println!("\n{}", table);

    Ok(())
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HistoricalPriceData {
    data: HistoricalAsset,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HistoricalAsset {
    base: String,
    currency: String,
    prices: Vec<HistoricalPrice>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HistoricalPrice {
    price: String,
    time: String,
}

#[cfg(test)]
pub mod test {

    use super::*;

    #[tokio::test]
    pub async fn midnight_price() -> Result<(), Box<dyn std::error::Error>> {
        let url = "https://api.coinbase.com/v2/prices/ETH-USD/historic?period=day";
        let prices: HistoricalPriceData = reqwest::Client::new()
            .get(url)
            .send()
            .await?
            .json::<HistoricalPriceData>()
            .await?;

        let recent = prices
            .data
            .prices
            .first()
            .unwrap()
            .price
            .parse::<f64>()
            .unwrap();

        let day_ago = prices
            .data
            .prices
            .last()
            .unwrap()
            .price
            .parse::<f64>()
            .unwrap();

        let percent_diff = ((recent - day_ago).abs() / ((recent + day_ago) / 2.0)) * 100.0;

        println!("{:?}", (recent, day_ago));
        println!("{:?}", percent_diff);

        Ok(())
    }
}
