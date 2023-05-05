use crate::error::Error;
use crate::state::State;
use clap::{Parser, Subcommand};
use miniscript::bitcoin;
use miniscript::bitcoin::hashes::sha256;
use miniscript::bitcoin::locktime::Height;
use miniscript::Descriptor;

mod address;
mod error;
mod image;
mod input;
mod key;
mod output;
mod spend;
mod state;
mod transaction;
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
    /// Key pair
    Key {
        #[command(subcommand)]
        key_command: KeyCommand,
    },
    /// Preimage-image pair
    Img {
        #[command(subcommand)]
        img_command: ImgCommand,
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
    ///
    /// Removes transaction inputs from UTXO set
    Final {
        /// Transaction id (hex)
        txid: bitcoin::Txid,
    },
}

#[derive(Subcommand)]
enum KeyCommand {
    /// Generate Schnorr key pairs
    ///
    /// Public keys are guaranteed to have an even y-coordinate (to work as x-only public keys)
    Gen {
        /// Number of pairs
        number: u32,
    },
    /// Enable key pair
    En {
        /// X-only public key
        key: bitcoin::XOnlyPublicKey,
    },
    /// Disable key pair
    Dis {
        /// X-only public key
        key: bitcoin::XOnlyPublicKey,
    },
    /// Delete key pair
    Del {
        /// X-only public key
        key: bitcoin::XOnlyPublicKey,
    },
}

#[derive(Subcommand)]
enum ImgCommand {
    /// Generate SHA-256 preimage-image pairs
    Gen {
        /// Number of pairs
        number: u32,
    },
    /// Enable preimage-image pair
    En {
        /// SHA-256 image
        image: sha256::Hash,
    },
    /// Disable preimage-image pair
    Dis {
        /// SHA-256 image
        image: sha256::Hash,
    },
    /// Delete preimage-image pair
    Del {
        /// SHA-256 image
        image: sha256::Hash,
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
        Commands::Key { key_command } => {
            let mut state = State::load(STATE_FILE_NAME)?;

            match key_command {
                KeyCommand::Gen { number } => {
                    key::generate_keys(&mut state, number)?;
                }
                KeyCommand::En { key } => {
                    key::enable_key(&mut state, key)?;
                }
                KeyCommand::Dis { key } => {
                    key::disable_key(&mut state, key)?;
                }
                KeyCommand::Del { key } => {
                    let old = key::delete_key(&mut state, &key)?;
                    println!("Deleting key pair {}", old.display_secret());
                }
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Img { img_command } => {
            let mut state = State::load(STATE_FILE_NAME)?;

            match img_command {
                ImgCommand::Gen { number } => {
                    image::generate_images(&mut state, number)?;
                }
                ImgCommand::En { image } => {
                    image::enable_image(&mut state, image)?;
                }
                ImgCommand::Dis { image } => {
                    image::disable_image(&mut state, image)?;
                }
                ImgCommand::Del { image } => {
                    let old = image::delete_image(&mut state, &image)?;
                    println!("Deleting preimage-image pair");
                    for byte in old {
                        print!("{:02x}", byte);
                    }
                    println!();
                }
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
        Commands::In { index, in_command } => {
            let mut state = State::load(STATE_FILE_NAME)?;

            match in_command {
                InCommand::New { utxo_index } => {
                    let old = input::add_from_utxo(&mut state, index, utxo_index)?;

                    if let Some(input) = old {
                        println!("Replacing input: {}", input);
                    }
                }
                InCommand::Del => {
                    let old = input::delete_input(&mut state, index)?;
                    println!("Deleting input: {}", old);
                }
                InCommand::Seq { seq_command } => match seq_command {
                    SeqCommand::Enable { relative_height } => {
                        input::update_sequence_height(&mut state, index, relative_height)?;
                    }
                    SeqCommand::Disable => {
                        input::set_sequence_max(&mut state, index)?;
                    }
                },
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Out { index, out_command } => {
            let mut state = State::load(STATE_FILE_NAME)?;

            match out_command {
                OutCommand::New { descriptor, value } => {
                    let old = output::add_output(&mut state, index, descriptor, value)?;

                    if let Some(output) = old {
                        println!("Replacing output: {}", output);
                    }
                }
                OutCommand::Del => {
                    let old = output::delete_output(&mut state, index)?;
                    println!("Deleting output: {}", old);
                }
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Locktime { height } => {
            let mut state = State::load(STATE_FILE_NAME)?;
            transaction::update_locktime(&mut state, height)?;
            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Fee { value } => {
            let mut state = State::load(STATE_FILE_NAME)?;
            transaction::update_fee(&mut state, value)?;
            state.save(STATE_FILE_NAME, false)?;
        }
        Commands::Spend => {
            let state = State::load(STATE_FILE_NAME)?;
            let (tx_hex, feerate) = spend::get_raw_transaction(&state)?;
            println!("Feerate: {:.2} sat / vB\n", feerate);
            println!("Send this transaction: {}", tx_hex);
        }
        Commands::Final { txid } => {
            let mut state = State::load(STATE_FILE_NAME)?;
            transaction::finalize_transaction(&mut state, txid)?;
            state.save(STATE_FILE_NAME, false)?;
        }
    }

    Ok(())
}
