use dotenv::dotenv;
use std::env;
use tonic_openssl_lnd::lnrpc::{AddInvoiceResponse, Invoice, PaymentHash};
use tonic_openssl_lnd::{LndClient, LndClientError};

pub async fn connect() -> Result<LndClient, LndClientError> {
    dotenv().ok();
    let port: u32 = env::var("LND_GRPC_PORT")
        .expect("LND_GRPC_PORT must be set")
        .parse()
        .expect("port is not u32");
    let host = env::var("LND_GRPC_HOST").expect("LND_GRPC_HOST must be set");
    let cert = env::var("LND_CERT_FILE").expect("LND_CERT_FILE must be set");
    let macaroon = env::var("LND_MACAROON_FILE").expect("LND_MACAROON_FILE must be set");
    // Connecting to LND requires only host, port, cert file, and macaroon file
    let client = tonic_openssl_lnd::connect(host, port, cert, macaroon)
        .await
        .expect("Failed connecting to LND");

    Ok(client)
}

pub async fn create_invoice(
    description: &str,
    amount: u32,
) -> Result<AddInvoiceResponse, LndClientError> {
    let mut client = connect().await.unwrap();
    let invoice = Invoice {
        memo: description.to_string(),
        value: amount as i64,
        ..Default::default()
    };
    let invoice = client.lightning().add_invoice(invoice).await?.into_inner();

    Ok(invoice)
}

pub async fn get_invoice(hash: &[u8]) -> Result<Invoice, LndClientError> {
    let mut client = connect().await.unwrap();
    let invoice = client
        .lightning()
        .lookup_invoice(PaymentHash {
            r_hash: hash.to_vec(),
            ..Default::default()
        })
        .await?
        .into_inner();

    Ok(invoice)
}
