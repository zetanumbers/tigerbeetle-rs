use std::time::{Duration, Instant};

use tigerbeetle::{Account, Client, Transfer};

const MAX_MESSAGE_BYTE_SIZE: usize = (1024 * 1024) - 128;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("TigerBeetle C Sample");
    println!("Connecting...");

    let client = Client::new(0, "127.0.0.1:3000", 32).expect("creating a tigerbeetle client");

    ////////////////////////////////////////////////////////////
    // Submitting a batch of accounts:                        //
    ////////////////////////////////////////////////////////////

    let accounts = [Account::new(1, 777, 2), Account::new(2, 777, 2)];
    let errors = client
        .create_accounts(accounts.into())
        .await
        .expect("creating accounts");
    assert!(errors.is_empty(), "failed to create accounts: {errors:?}");
    println!("Accounts created successfully");

    ////////////////////////////////////////////////////////////
    // Submitting multiple batches of transfers:              //
    ////////////////////////////////////////////////////////////

    println!("Creating transfers...");
    const MAX_BATCHES: usize = 100;
    const TRANSFERS_PER_BATCH: usize = MAX_MESSAGE_BYTE_SIZE / std::mem::size_of::<Transfer>();
    let mut max_latency = Duration::ZERO;
    let mut total_time = Duration::ZERO;

    for i in 0..MAX_BATCHES {
        let transfers = (0..TRANSFERS_PER_BATCH)
            .map(|j| Transfer {
                id: (j + 1 + i * TRANSFERS_PER_BATCH).try_into().unwrap(),
                debit_account_id: accounts[0].id(),
                credit_account_id: accounts[1].id(),
                code: 2,
                ledger: 777,
                amount: 1,
                ..bytemuck::Zeroable::zeroed()
            })
            .collect();

        let start = Instant::now();
        let errors = client
            .create_transfers(transfers)
            .await
            .expect("creating transfers");
        assert!(errors.is_empty(), "failed to create transfers: {errors:?}");

        let elapsed = start.elapsed();
        max_latency = max_latency.max(elapsed);
        total_time += elapsed;
    }

    println!("Transfers created successfully");
    println!("============================================");
    println!(
        "{:.0} transfers per second",
        (MAX_BATCHES * TRANSFERS_PER_BATCH) as f64 / total_time.as_secs_f64()
    );
    println!(
        "create_transfers max p100 latency per {} transfers = {}ms",
        TRANSFERS_PER_BATCH,
        max_latency.as_millis()
    );
    println!(
        "total {} transfers in {}ms",
        MAX_BATCHES * TRANSFERS_PER_BATCH,
        total_time.as_millis()
    );
    println!();

    ////////////////////////////////////////////////////////////
    // Looking up accounts:                                   //
    ////////////////////////////////////////////////////////////

    println!("Looking up accounts ...");
    let ids = accounts.map(|a| a.id());
    let accounts = client
        .lookup_accounts(ids.into())
        .await
        .expect("looking up accounts");
    assert!(!accounts.is_empty());
    println!("{} Account(s) found", accounts.len());
    println!("============================================");
    for account in accounts {
        println!(
            "Account {{ id: {}, debits_posted: {}, credits_posted: {}, .. }}",
            account.id(),
            account.debits_posted(),
            account.credits_posted()
        );
    }
}
