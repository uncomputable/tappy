use crate::error::Error;
use crate::state::State;
use clap::{Parser, Subcommand};
use miniscript::bitcoin::{Txid, XOnlyPublicKey};
use miniscript::Descriptor;

mod error;
mod fund;
mod spend;
mod state;
mod update;

const STATE_FILE_NAME: &str = "state.json";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create an empty state and save it to `state.json`
    ///
    /// Fails if file already exists
    Init,
    /// Print current state
    Print,
    /// Generate a set of keypairs
    ///
    /// Public keys are guaranteed to have an even y-coordinate (to work as x-only public keys)
    Keygen {
        /// Number of generated keypairs
        number: u32,
    },
    /// Activate a passive key or passivize an active key
    Trigger {
        /// X-only public key from current state
        pubkey: XOnlyPublicKey,
    },
    /// Get address of transaction input to fund it via bitcoind
    Fund {
        /// Input index
        index: usize,
    },
    /// Create transaction witness and print raw transaction hex to broadcast via bitcoind
    Spend,
    /// Add transaction input
    In {
        /// Input index
        index: usize,
        /// Descriptor
        descriptor: Descriptor<XOnlyPublicKey>,
    },
    /// Add transaction output
    Out {
        /// Output index
        index: usize,
        /// Descriptor
        descriptor: Descriptor<XOnlyPublicKey>,
        /// Output value in satoshi
        ///
        /// Zero satoshi means that the output will receive the remaining input funds
        /// (inputs minus outputs minus fee).
        /// This is possible for AT MOST ONE output!
        #[arg(default_value_t = 0)]
        value: u64,
    },
    /// Add UTXO for a transaction input
    Utxo {
        /// Corresponding input index
        input_index: usize,
        /// UTXO transaction id (hex)
        txid: Txid,
        /// Output index (vout)
        output_index: u32,
        /// Output value in satoshi
        value: u64,
    },
    /// Update the transaction fee
    Fee {
        /// Transaction fee in satoshi
        value: u64,
    },
    /// Convert transaction output into transaction input
    ///
    /// This is useful for creating a new transaction that is funded by the previous transaction
    Move {
        /// Output index
        output_index: usize,
        /// Input index
        input_index: usize,
    },
}

// TODO: Add preimages
// TODO: Add locktime
// TODO: Add sequence
fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        None => {}
        Some(Commands::Init) => {
            let state = State::new();
            state.save(STATE_FILE_NAME, true)?;
        }
        Some(Commands::Print) => {
            let state = State::load(STATE_FILE_NAME)?;
            println!("{}", state);
        }
        Some(Commands::Keygen { number }) => {
            let mut state = State::load(STATE_FILE_NAME)?;
            update::generate_unknown_keys(&mut state, number)?;
            state.save(STATE_FILE_NAME, false)?;
        }
        Some(Commands::Trigger { pubkey }) => {
            let mut state = State::load(STATE_FILE_NAME)?;
            update::trigger_key(&mut state, pubkey)?;
            state.save(STATE_FILE_NAME, false)?;
        }
        Some(Commands::Fund { index }) => {
            let state = State::load(STATE_FILE_NAME)?;
            let address = fund::get_input_address(&state, index)?;
            println!("Input address: {}", address);
        }
        Some(Commands::Spend) => {
            let state = State::load(STATE_FILE_NAME)?;
            let (tx_hex, feerate) = spend::get_raw_transaction(&state)?;
            println!("Feerate: {:.2} sat / vB\n", feerate);
            println!("Spending tx hex: {}", tx_hex);
        }
        Some(Commands::In { index, descriptor }) => {
            let mut state = State::load(STATE_FILE_NAME)?;
            let old = update::add_input(&mut state, index, descriptor)?;

            if let Some(input) = old {
                println!("Replacing old input: {}", input);
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Some(Commands::Out {
            index,
            descriptor,
            value,
        }) => {
            let mut state = State::load(STATE_FILE_NAME)?;
            let old = update::add_output(&mut state, index, descriptor, value)?;

            if let Some(output) = old {
                println!("Replacing old output: {}", output);
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Some(Commands::Utxo {
            input_index,
            txid,
            output_index,
            value,
        }) => {
            let mut state = State::load(STATE_FILE_NAME)?;
            let old = update::add_utxo(&mut state, input_index, txid, output_index, value)?;

            if let Some(utxo) = old {
                println!("Replacing old UTXO: {}", utxo);
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Some(Commands::Fee { value }) => {
            let mut state = State::load(STATE_FILE_NAME)?;
            update::update_fee(&mut state, value)?;
            state.save(STATE_FILE_NAME, false)?;
        }
        Some(Commands::Move {
            output_index,
            input_index,
        }) => {
            let mut state = State::load(STATE_FILE_NAME)?;
            let old = update::move_output(&mut state, output_index, input_index)?;

            if let Some(input) = old {
                println!("Replacing old input: {}", input);
            }

            state.save(STATE_FILE_NAME, false)?;
        }
    }

    Ok(())
}
