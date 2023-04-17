use crate::error::Error;
use crate::state::State;
use clap::{Parser, Subcommand};
use miniscript::bitcoin;
use miniscript::bitcoin::hashes::sha256;
use miniscript::bitcoin::locktime::Height;
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
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create an empty state and save it to `state.json`
    ///
    /// Fails if file already exists
    Init,
    /// Print current state
    Print,
    /// Generate random state
    Gen {
        /// Generate a set of key pairs
        ///
        /// Public keys are guaranteed to have an even y-coordinate (to work as x-only public keys)
        #[arg(short, long, group = "mode")]
        keys: bool,
        /// Generate a set of image-preimage pairs
        #[arg(short, long, group = "mode")]
        images: bool,
        /// Number of generated pairs
        #[arg(requires = "mode")]
        number: u32,
    },
    /// Activate a passive item or passivize an active one from current state
    #[group(required = true)]
    Toggle {
        /// X-only public key
        #[arg(short, long)]
        key: Option<bitcoin::XOnlyPublicKey>,
        /// SHA-256 hash
        #[arg(short, long)]
        image: Option<sha256::Hash>,
    },
    /// Get address of transaction input to fund via Bitcoin Core
    Fund {
        /// Input index
        index: usize,
    },
    /// Create transaction witness and print raw transaction hex to send via Bitcoin Core
    Spend,
    /// Add transaction input
    In {
        /// Input index
        index: usize,
        /// Descriptor
        descriptor: Descriptor<bitcoin::XOnlyPublicKey>,
    },
    /// Add transaction output
    Out {
        /// Output index
        index: usize,
        /// Descriptor
        descriptor: Descriptor<bitcoin::XOnlyPublicKey>,
        /// Output value in satoshi
        ///
        /// Zero satoshi means that the output will receive the remaining input funds
        /// (inputs minus outputs minus fee)
        ///
        /// This is possible for at most one input!
        #[arg(default_value_t = 0)]
        value: u64,
    },
    /// Add UTXO for a transaction input
    Utxo {
        /// Corresponding input index
        input_index: usize,
        /// UTXO transaction id (hex)
        txid: bitcoin::Txid,
        /// Output index (vout)
        output_index: u32,
        /// Output value in satoshi
        value: u64,
    },
    /// Update locktime
    Locktime {
        /// Absolute block height
        ///
        /// A transaction is valid if the current block height
        /// is greater than the transaction's locktime
        ///
        /// To enable a transaction's locktime,
        /// at least one of its inputs must have a relative timelock
        /// (which may be zero)!
        ///
        /// Enabling locktime without relative timelock is not supported
        height: Height,
    },
    /// Update sequence of a transaction input
    Seq {
        /// Input index
        input_index: usize,
        /// Relative timelock
        #[clap(subcommand)]
        relative_locktime: RelativeLocktime,
    },
    /// Update transaction fee
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

#[derive(Subcommand)]
enum RelativeLocktime {
    /// Enable relative timelock for this input
    Enable {
        /// Relative block height
        ///
        /// A transaction input is valid if the current block height
        /// is greater than the utxo height plus the input's relative locktime
        ///
        /// A transaction is valid if all its inputs are valid
        #[arg(default_value_t = 0)]
        relative_height: u16,
    },
    /// Disable relative timelock for this input
    Disable,
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            let state = State::new();
            state.save(STATE_FILE_NAME, true)?;
        }
        Commands::Print => {
            let state = State::load(STATE_FILE_NAME)?;
            println!("{}", state);
        }
        Commands::Gen {
            keys,
            images,
            number,
        } => {
            let mut state = State::load(STATE_FILE_NAME)?;

            if keys {
                update::generate_keys(&mut state, number)?;
            }
            if images {
                update::generate_images(&mut state, number)?;
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Toggle { key, image } => {
            let mut state = State::load(STATE_FILE_NAME)?;

            if let Some(pubkey) = key {
                update::toggle_key(&mut state, pubkey)?;
            }
            if let Some(image) = image {
                update::toggle_image(&mut state, image)?;
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Fund { index } => {
            let state = State::load(STATE_FILE_NAME)?;
            let address = fund::get_input_address(&state, index)?;
            println!("Input address: {}", address);
        }
        Commands::Spend => {
            let state = State::load(STATE_FILE_NAME)?;
            let (tx_hex, feerate) = spend::get_raw_transaction(&state)?;
            println!("Feerate: {:.2} sat / vB\n", feerate);
            println!("Spending tx hex: {}", tx_hex);
        }
        Commands::In { index, descriptor } => {
            let mut state = State::load(STATE_FILE_NAME)?;
            let old = update::add_input(&mut state, index, descriptor)?;

            if let Some(input) = old {
                println!("Replacing old input: {}", input);
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Out {
            index,
            descriptor,
            value,
        } => {
            let mut state = State::load(STATE_FILE_NAME)?;
            let old = update::add_output(&mut state, index, descriptor, value)?;

            if let Some(output) = old {
                println!("Replacing old output: {}", output);
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Utxo {
            input_index,
            txid,
            output_index,
            value,
        } => {
            let mut state = State::load(STATE_FILE_NAME)?;
            let old = update::add_utxo(&mut state, input_index, txid, output_index, value)?;

            if let Some(utxo) = old {
                println!("Replacing old UTXO: {}", utxo);
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Locktime { height } => {
            let mut state = State::load(STATE_FILE_NAME)?;
            update::update_locktime(&mut state, height)?;
            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Seq {
            input_index,
            relative_locktime,
        } => {
            let mut state = State::load(STATE_FILE_NAME)?;

            match relative_locktime {
                RelativeLocktime::Enable { relative_height } => {
                    update::update_sequence_height(&mut state, input_index, relative_height)?;
                }
                RelativeLocktime::Disable => {
                    update::set_sequence_max(&mut state, input_index)?;
                }
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Fee { value } => {
            let mut state = State::load(STATE_FILE_NAME)?;
            update::update_fee(&mut state, value)?;
            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Move {
            output_index,
            input_index,
        } => {
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
