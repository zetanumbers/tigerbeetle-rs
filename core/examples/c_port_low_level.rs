use std::{
    io::Write,
    mem,
    sync::{Condvar, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use tigerbeetle_unofficial_core as tb;

const MAX_MESSAGE_SIZE: usize = (1024 * 1024) - 256;

struct Callbacks;

struct UserData {
    ctx: &'static CompletionContext,
    data: [u8; MAX_MESSAGE_SIZE],
    data_size: usize,
}

// Synchronization context between the callback and the main thread.
// In this example we synchronize using a condition variable.
struct CompletionContext {
    state: Mutex<CompletionState>,
    cv: Condvar,
}

struct CompletionState {
    reply: [u8; MAX_MESSAGE_SIZE],
    size: usize,
    completed: Option<(Box<UserData>, Result<(), tb::error::SendError>)>,
}

fn main() {
    println!("TigerBeetle C Sample");
    println!("Connecting...");
    let address = std::env::var("TB_ADDRESS");
    let address = address.as_deref().unwrap_or("3000");
    let client = tb::Client::with_callback(0, address.as_bytes(), 32, &Callbacks)
        .expect("Failed to initialize tigerbeetle client");

    static CTX: CompletionContext = CompletionContext::new();

    ////////////////////////////////////////////////////////////
    // Submitting a batch of accounts:                        //
    ////////////////////////////////////////////////////////////

    let accounts = [tb::Account::new(1, 777, 2), tb::Account::new(2, 777, 2)];
    let mut user_data = Box::new(UserData {
        ctx: &CTX,
        data: [0; MAX_MESSAGE_SIZE],
        data_size: 0,
    });
    user_data.set_data(accounts);
    let mut packet = client
        .acquire(user_data, tb::OperationKind::CreateAccounts.into())
        .unwrap();
    println!("Creating accounts...");
    let mut state = CTX.state.lock().unwrap();
    (user_data, state) = CTX.send_request(state, packet).unwrap();
    state.create_accounts_status().unwrap();

    println!("Accounts created successfully");

    ////////////////////////////////////////////////////////////
    // Submitting multiple batches of transfers:              //
    ////////////////////////////////////////////////////////////

    println!("Creating transfers...");
    const MAX_BATCHES: usize = 100;
    const TRANSFERS_PER_BATCH: usize = MAX_MESSAGE_SIZE / mem::size_of::<tb::Transfer>();
    let max_batches = std::env::var("TIGERBEETLE_RS_MAX_BATCHES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(MAX_BATCHES);
    let mut max_latency = Duration::ZERO;
    let mut total_time = Duration::ZERO;

    for i in 0..max_batches {
        let transfers = (0..TRANSFERS_PER_BATCH).map(|j| {
            tb::Transfer::new((j + 1 + (i * TRANSFERS_PER_BATCH)) as u128)
                .with_debit_account_id(accounts[0].id())
                .with_credit_account_id(accounts[1].id())
                .with_code(2)
                .with_ledger(777)
                .with_amount(1)
        });
        user_data.set_data(transfers);
        packet = client
            .acquire(user_data, tb::OperationKind::CreateTransfers.into())
            .unwrap();

        let now = Instant::now();
        (user_data, state) = CTX.send_request(state, packet).unwrap();
        let elapsed = now.elapsed();
        max_latency = max_latency.max(elapsed);
        total_time += elapsed;

        state.create_transfers_status().unwrap();
    }

    println!("Transfers created successfully");
    println!("============================================");

    println!(
        "{} transfers per second\n",
        (max_batches * TRANSFERS_PER_BATCH * 1000)
            / usize::try_from(total_time.as_millis()).unwrap()
    );
    println!(
        "create_transfers max p100 latency per {} transfers = {}ms",
        TRANSFERS_PER_BATCH,
        max_latency.as_millis()
    );
    println!(
        "total {} transfers in {}ms",
        max_batches * TRANSFERS_PER_BATCH,
        total_time.as_millis()
    );
    println!();

    ////////////////////////////////////////////////////////////
    // Looking up accounts:                                   //
    ////////////////////////////////////////////////////////////

    println!("Looking up accounts ...");
    let ids = accounts.map(|a| a.id());
    user_data.set_data(ids);
    packet = client
        .acquire(user_data, tb::OperationKind::LookupAccounts.into())
        .unwrap();
    (_, state) = CTX.send_request(state, packet).unwrap();
    let accounts = state.get_data::<tb::Account>();
    if accounts.is_empty() {
        panic!("No accounts found");
    }

    // Printing the account's balance:
    println!("{} Account(s) found", accounts.len());
    println!("============================================");
    println!("{accounts:#?}");
}

impl CompletionContext {
    const fn new() -> Self {
        CompletionContext {
            state: Mutex::new(CompletionState {
                reply: [0; MAX_MESSAGE_SIZE],
                size: 0,
                completed: None,
            }),
            cv: Condvar::new(),
        }
    }

    fn send_request<'a>(
        &self,
        mut guard: MutexGuard<'a, CompletionState>,
        packet: tb::Packet<Box<UserData>>,
    ) -> Result<(Box<UserData>, MutexGuard<'a, CompletionState>), tb::error::SendError> {
        guard.completed = None;
        packet.submit();
        loop {
            guard = self.cv.wait(guard).unwrap();

            if let Some(c) = guard.completed.take() {
                break c.1.map(|()| (c.0, guard));
            }
        }
    }
}

impl CompletionState {
    fn create_accounts_status(&self) -> Result<(), tb::error::CreateAccountsApiError> {
        match tb::error::CreateAccountsApiError::from_raw_results(self.get_data()) {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn create_transfers_status(&self) -> Result<(), tb::error::CreateTransfersApiError> {
        match tb::error::CreateTransfersApiError::from_raw_results(self.get_data()) {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn get_data<T>(&self) -> Vec<T>
    where
        T: bytemuck::Pod,
    {
        if self.size == 0 {
            return Vec::new();
        }
        assert_eq!(self.size % mem::size_of::<T>(), 0);
        let mut res = vec![T::zeroed(); self.size / mem::size_of::<T>()];
        bytemuck::cast_slice_mut::<_, u8>(&mut res).copy_from_slice(&self.reply[..self.size]);
        res
    }
}

impl tb::UserData for UserData {
    fn data(&self) -> &[u8] {
        &self.data[..self.data_size]
    }
}

impl UserData {
    fn set_data<I>(&mut self, src: I)
    where
        I: IntoIterator,
        I::Item: bytemuck::Pod,
    {
        let mut dst = self.data.as_mut_slice();
        for src in src {
            dst.write_all(bytemuck::bytes_of(&src)).unwrap();
        }
        self.data_size = MAX_MESSAGE_SIZE - dst.len();
    }

    fn free(self: Box<Self>, status: Result<(), tb::error::SendError>) {
        let mut l = self.ctx.state.lock().unwrap();
        l.completed = Some((self, status));
    }
}

impl tb::Callbacks for Callbacks {
    type UserDataPtr = Box<UserData>;

    fn on_completion(&self, packet: tb::Packet<'_, Self::UserDataPtr>, payload: &[u8]) {
        let status = packet.status();
        let user_data = packet.into_user_data();
        let ctx = user_data.ctx;
        {
            let mut state = ctx.state.lock().unwrap();
            state.reply[..payload.len()].copy_from_slice(payload);
            state.size = payload.len();
            ctx.cv.notify_one();
        }
        user_data.free(status);
    }
}
