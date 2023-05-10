use crate::descriptor::SimplicityDescriptor;
use crate::error::Error;
use crate::state::State;
use clap::{Parser, Subcommand};
use elements_miniscript::bitcoin::hashes::sha256;
use elements_miniscript::{bitcoin, elements};

mod address;
mod descriptor;
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
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create empty state
    ///
    /// Fails if file already exists
    Init,
    /// Print current state
    Print,
    /// Schnorr key pair
    Key {
        #[command(subcommand)]
        key_command: KeyCommand,
    },
    /// SHA-256 (pre)image pair
    Img {
        #[command(subcommand)]
        img_command: ImgCommand,
    },
    /// Temporary inbound address for creating UTXOs
    Addr {
        #[clap(subcommand)]
        addr_command: AddrCommand,
    },
    /// UTXO (unspent transaction output)
    Utxo {
        #[clap(subcommand)]
        utxo_command: UtxoCommand,
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
    /// Update locktime
    Locktime {
        /// Absolute block height
        ///
        /// A transaction is valid if its containing block height
        /// is strictly greater than its locktime
        ///
        /// To enable locktime,
        /// at least one of the inputs must have a relative locktime
        /// (which may be zero)!
        ///
        /// Other ways to enable locktime are not supported
        // TODO: Replace with elements::locktime::Height once FromStr::Error implements std::error::Error
        //
        height: u32,
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
        txid: elements::Txid,
    },
}

#[derive(Subcommand)]
enum KeyCommand {
    /// Generate key pairs
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
    /// Generate (pre)image pairs
    Gen {
        /// Number of pairs
        number: u32,
    },
    /// Enable (pre)image pair
    En {
        /// SHA-256 image
        image: sha256::Hash,
    },
    /// Disable (pre)image pair
    Dis {
        /// SHA-256 image
        image: sha256::Hash,
    },
    /// Delete (pre)image pair
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
        descriptor: SimplicityDescriptor<bitcoin::XOnlyPublicKey>,
    },
    /// Convert inbound address into UTXO
    Utxo {
        /// UTXO transaction id (hex)
        txid: elements::Txid,
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
        descriptor: SimplicityDescriptor<bitcoin::XOnlyPublicKey>,
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
    /// Enable relative locktime for this input
    Enable {
        /// Relative block height
        ///
        /// An input is valid if its containing block height
        /// is strictly greater than the UTXO height plus the input's relative locktime
        ///
        /// A transaction is valid if all its inputs are valid
        #[arg(default_value_t = 0)]
        relative_height: u16,
    },
    /// Disable relative locktime for this input
    Disable,
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => {
            let state = State::new();
            println!("Generating state.json");
            state.save(STATE_FILE_NAME, true)?;
        }
        Command::Print => {
            let state = State::load(STATE_FILE_NAME)?;
            println!("{}", state);
        }
        Command::Key { key_command } => {
            let mut state = State::load(STATE_FILE_NAME)?;

            match key_command {
                KeyCommand::Gen { number } => {
                    key::generate_keys(&mut state, number)?;
                }
                KeyCommand::En { key } => {
                    key::enable_key(&mut state, key)?;
                    println!("Enabling key: {}", key);
                }
                KeyCommand::Dis { key } => {
                    key::disable_key(&mut state, key)?;
                    println!("Disabling key: {}", key);
                }
                KeyCommand::Del { key } => {
                    let old = key::delete_key(&mut state, &key)?;
                    println!("Deleting key pair: {}", old.display_secret());
                }
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Command::Img { img_command } => {
            let mut state = State::load(STATE_FILE_NAME)?;

            match img_command {
                ImgCommand::Gen { number } => {
                    image::generate_images(&mut state, number)?;
                }
                ImgCommand::En { image } => {
                    image::enable_image(&mut state, image)?;
                    println!("Enabling image: {}", image);
                }
                ImgCommand::Dis { image } => {
                    image::disable_image(&mut state, image)?;
                    println!("Disabling image: {}", image);
                }
                ImgCommand::Del { image } => {
                    let old = image::delete_image(&mut state, &image)?;
                    print!("Deleting (pre)image pair: ");
                    for byte in old {
                        print!("{:02x}", byte);
                    }
                    println!();
                }
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Command::Addr { addr_command } => {
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
        Command::Utxo { utxo_command } => {
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
        Command::In { index, in_command } => {
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
                        let locktime_before = state.locktime_enabled();
                        input::update_sequence_height(&mut state, index, relative_height)?;
                        println!("Relative timelock: +{} blocks", relative_height);

                        if !locktime_before {
                            println!("Locktime: enabled");
                        }
                    }
                    SeqCommand::Disable => {
                        input::set_sequence_max(&mut state, index)?;
                        println!("Relative timelock: disabled");

                        if !state.locktime_enabled() {
                            println!("Locktime: disabled");
                        }
                    }
                },
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Command::Out { index, out_command } => {
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
        Command::Locktime { height } => {
            let mut state = State::load(STATE_FILE_NAME)?;
            transaction::update_locktime(&mut state, height)?;
            println!("Locktime: ={} blocks", height);

            if !state.locktime_enabled() {
                println!("Locktime: disabled (enable via input sequence)");
            }

            state.save(STATE_FILE_NAME, false)?;
        }
        Command::Fee { value } => {
            let mut state = State::load(STATE_FILE_NAME)?;
            transaction::update_fee(&mut state, value)?;
            println!("Fee: {} sat", value);
            state.save(STATE_FILE_NAME, false)?;
        }
        Command::Spend => {
            let mut state = State::load(STATE_FILE_NAME)?;
            let (tx_hex, feerate) = spend::get_raw_transaction(&mut state)?;
            println!("Feerate: {:.2} sat / vB\n", feerate);
            println!("Send this transaction: {}", tx_hex);
            state.save(STATE_FILE_NAME, false)?;
        }
        Command::Final { txid } => {
            let mut state = State::load(STATE_FILE_NAME)?;
            transaction::finalize_transaction(&mut state, txid)?;
            state.save(STATE_FILE_NAME, false)?;
        }
    }

    Ok(())
}
