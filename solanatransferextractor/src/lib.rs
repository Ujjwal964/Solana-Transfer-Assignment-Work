#[allow(unused)]
mod pb;

use pb::{
    mydata::v1 as mydata,
    sf::{
        solana::r#type::v1::{CompiledInstruction, InnerInstruction},
        substreams::solana::v1::Transactions,
    },
};

const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

fn add_transfer(
    result: &mut mydata::MyData,
    keys: &[Vec<u8>],
    program_id_index: u32,
    accounts: &[u8],
    data: &[u8],
) {
    let program_id = match keys.get(program_id_index as usize) {
        Some(key) => bs58::encode(key).into_string(),
        None => return,
    };

    let (from_index, to_index, amount) = if program_id == SYSTEM_PROGRAM {
        // System Program Transfer:
        // 4-byte instruction discriminator (2), followed by u64 lamports.
        if data.len() < 12
            || u32::from_le_bytes(data[0..4].try_into().unwrap()) != 2
            || accounts.len() < 2
        {
            return;
        }

        (
            accounts[0],
            accounts[1],
            u64::from_le_bytes(data[4..12].try_into().unwrap()),
        )
    } else if program_id == TOKEN_PROGRAM || program_id == TOKEN_2022_PROGRAM {
        if data.len() < 9 {
            return;
        }

        let instruction_type = data[0];

        let (from, to) = match instruction_type {
            3 if accounts.len() >= 2 => (accounts[0], accounts[1]),
            12 if accounts.len() >= 3 => (accounts[0], accounts[2]),
            _ => return,
        };

        (from, to, u64::from_le_bytes(data[1..9].try_into().unwrap()))
    } else {
        return;
    };

    let from = match keys.get(from_index as usize) {
        Some(key) => bs58::encode(key).into_string(),
        None => return,
    };

    let to = match keys.get(to_index as usize) {
        Some(key) => bs58::encode(key).into_string(),
        None => return,
    };

    result.transfers.push(mydata::Transfer {
        from,
        to,
        amount,
    });
}

fn build_account_keys(
    static_keys: &[Vec<u8>],
    loaded_writable: &[Vec<u8>],
    loaded_readonly: &[Vec<u8>],
) -> Vec<Vec<u8>> {
    let mut keys = static_keys.to_vec();
    keys.extend_from_slice(loaded_writable);
    keys.extend_from_slice(loaded_readonly);
    keys
}

fn process_compiled(
    result: &mut mydata::MyData,
    keys: &[Vec<u8>],
    instruction: &CompiledInstruction,
) {
    add_transfer(
        result,
        keys,
        instruction.program_id_index,
        &instruction.accounts,
        &instruction.data,
    );
}

fn process_inner(
    result: &mut mydata::MyData,
    keys: &[Vec<u8>],
    instruction: &InnerInstruction,
) {
    add_transfer(
        result,
        keys,
        instruction.program_id_index,
        &instruction.accounts,
        &instruction.data,
    );
}

#[substreams::handlers::map]
fn map_my_data(transactions: Transactions) -> mydata::MyData {
    let mut result = mydata::MyData::default();

    for confirmed_tx in &transactions.transactions {
        let tx = match confirmed_tx.transaction.as_option() {
            Some(tx) => tx,
            None => continue,
        };

        let message = match tx.message.as_option() {
            Some(message) => message,
            None => continue,
        };

        let meta = confirmed_tx.meta.as_option();

        // Ignore failed transactions: their instructions did not commit.
        if let Some(meta) = meta {
            if meta.err.is_set() {
                continue;
            }
        } else {
            continue;
        }

        let meta = meta.unwrap();

        let keys = build_account_keys(
            &message.account_keys,
            &meta.loaded_writable_addresses,
            &meta.loaded_readonly_addresses,
        );

        // Top-level instructions and their corresponding inner instructions.
        for (index, instruction) in message.instructions.iter().enumerate() {
            process_compiled(&mut result, &keys, instruction);

            if let Some(inner_group) = meta
                .inner_instructions
                .iter()
                .find(|group| group.index == index as u32)
            {
                for inner in &inner_group.instructions {
                    process_inner(&mut result, &keys, inner);
                }
            }
        }
    }

    result
}