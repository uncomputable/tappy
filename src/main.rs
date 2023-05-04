use crate::error::Error;
use crate::state::State;
use clap::{Parser, Subcommand};
use miniscript::bitcoin;
use miniscript::bitcoin::hashes::sha256;
use miniscript::bitcoin::locktime::Height;
use miniscript::Descriptor;

mod address;
mod error;
mod spend;
mod state;
mod update;
mod util;
mod utxo;

const STATE_FILE_NAME: &str = "state.json";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create empty state
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
    /// (De)activate an item from current state
    #[group(required = true)]
    Toggle {
        /// X-only public key
        #[arg(short, long)]
        key: Option<bitcoin::XOnlyPublicKey>,
        /// SHA-256 image
        #[arg(short, long)]
        image: Option<sha256::Hash>,
    },
    /// Temporary inbound address for creating UTXOs
    Addr {
        #[clap(subcommand)]
        addr_command: AddrCommand,
    },
    /// Transaction input
    In {
        /// Input index
        index: usize,
        #[clap(subcommand)]
        in_command: InCommand,
    },
    /// Transaction output
    Out {
        /// Output index
        index: usize,
        #[clap(subcommand)]
        out_command: OutCommand,
    },
    /// UTXO (unspent transaction output)
    Utxo {
        #[clap(subcommand)]
        utxo_command: UtxoCommand,
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
    /// Update transaction fee
    Fee {
        /// Transaction fee in satoshi
        value: u64,
    },
    /// Create transaction witness and print raw transaction hex to send via Bitcoin Core
    Spend,
    /// Finalize transaction and save transaction outputs as UTXOs
    ///
    /// Creates new transaction with first transaction output as input
    Final {
        /// Transaction id (hex)
        txid: bitcoin::Txid,
    },
}

#[derive(Subcommand)]
enum AddrCommand {
    /// Set inbound address to fund via Bitcoin Core
    Set {
        /// Descriptor
        descriptor: Descriptor<bitcoin::XOnlyPublicKey>,
    },
    /// Convert inbound address into UTXO
    Utxo {
        /// UTXO transaction id (hex)
        txid: bitcoin::Txid,
        /// Output index (vout)
        output_index: u32,
        /// Output value in satoshi
        value: u64,
    },
}

#[derive(Subcommand)]
enum UtxoCommand {
    /// List UTXOs with their index
    List,
    /// Delete UTXO
    Del {
        /// UTXO index
        utxo_index: usize,
    },
}

#[derive(Subcommand)]
enum InCommand {
    /// Add new transaction input
    New {
        /// UTXO index
        utxo_index: usize,
    },
    /// Delete transaction input
    Del,
    /// Update sequence of transaction input
    Seq {
        #[clap(subcommand)]
        seq_command: SeqCommand,
    },
}

#[derive(Subcommand)]
enum OutCommand {
    /// Add new transaction output
    New {
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
    /// Delete transaction output
    Del,
}

#[derive(Subcommand)]
enum SeqCommand {
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
        Commands::Addr { addr_command } => {
            let mut state = State::load(STATE_FILE_NAME)?;

            match addr_command {
                AddrCommand::Set { descriptor } => {
                    let address = address::set_address(&mut state, descriptor)?;
                    println!("Fund this address: {}", address);
                }
                AddrCommand::Utxo {
                    txid,
                    output_index,
                    value,
                } => {
                    address::into_utxo(&mut state, txid, output_index, value)?;
                }
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Utxo { utxo_command } => {
            let mut state = State::load(STATE_FILE_NAME)?;

            match utxo_command {
                UtxoCommand::List => {
                    utxo::list_utxos(&state);
                }
                UtxoCommand::Del { utxo_index } => {
                    let old = utxo::delete_utxo(&mut state, utxo_index)?;
                    println!("Deleting UTXO: {}", old);
                }
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        /*
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
         */
        Commands::Locktime { height } => {
            let mut state = State::load(STATE_FILE_NAME)?;
            update::update_locktime(&mut state, height)?;
            state.save(STATE_FILE_NAME, false)?;
        }
        /*
        Commands::Seq {
            input_index,
            relative_locktime,
        } => {
            let mut state = State::load(STATE_FILE_NAME)?;

            match relative_locktime {
                SeqCommand::Enable { relative_height } => {
                    update::update_sequence_height(&mut state, input_index, relative_height)?;
                }
                SeqCommand::Disable => {
                    update::set_sequence_max(&mut state, input_index)?;
                }
            }

            state.save(STATE_FILE_NAME, false)?;
        }
         */
        Commands::Fee { value } => {
            let mut state = State::load(STATE_FILE_NAME)?;
            update::update_fee(&mut state, value)?;
            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Spend => {
            let state = State::load(STATE_FILE_NAME)?;
            let (tx_hex, feerate) = spend::get_raw_transaction(&state)?;
            println!("Feerate: {:.2} sat / vB\n", feerate);
            println!("Spending tx hex: {}", tx_hex);
        }
        _ => {}
    }

    Ok(())
}
