use alloy::{
    primitives::{Address, U256, B256, Bytes, TxKind},
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    network::{EthereumWallet, TxSigner},
    consensus::{TxLegacy, TxEnvelope, SignableTransaction},
    eips::eip2718::Encodable2718,
    sol,
};
use serde_json::json;
use std::env;
use std::str::FromStr;
use anyhow::Result;

sol!(
    #[sol(rpc)]
    contract FourMemeRouter {
        function buyTokenAMAP(address token, uint256 funds, uint256 minAmount)
            external
            payable;
        
        // Keeping the old one just in case, or for reference
        struct SwapDesc {
            uint8 swapType;
            address tokenIn;
            address tokenOut;
            address poolAddress;
            uint24 fee;
            int24 tickSpacing;
            address hooks;
            bytes hookData;
            address poolManager;
            bytes32 parameters;
        }

        function swap(SwapDesc[] memory descs, address feeToken, uint256 amountIn, uint256 minReturn)
            external
            payable;
    }
);

pub async fn simulate_swap(token_address_str: &str) -> Result<()> {
    let rpc_url = env::var("BSC_RPC_URL")?;
    let private_key = env::var("PRIVATE_KEY")?;
    let buy_amount_bnb = env::var("BUY_AMOUNT_BNB")?.parse::<f64>()?;
    let router_address_str = env::var("PANCAKE_ROUTER")?;
    let router_address = Address::from_str(&router_address_str)?;
    let token_address = Address::from_str(token_address_str)?;

    let signer: PrivateKeySigner = private_key.parse()?;
    let wallet = EthereumWallet::from(signer.clone());

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_builtin(&rpc_url)
        .await?;

    let funds = U256::from((buy_amount_bnb * 1e18) as u64);
    
    println!("Simulating buyTokenAMAP for {} BNB of token {}", buy_amount_bnb, token_address);

    let router = FourMemeRouter::new(router_address, provider.clone());

    let call = router.buyTokenAMAP(
        token_address,
        funds,
        U256::from(0) 
    );

    let calldata = format!("0x{}", hex::encode(call.calldata()));
    println!("Raw Calldata: {}", calldata);
    println!("From Address: {:?}", signer.address());
    println!("Value:        {} wei", funds);

    // Raw JSON-RPC eth_call
    let params = json!([
        {
            "from": signer.address(),
            "to": router_address,
            "value": format!("0x{:x}", funds),
            "data": calldata
        },
        "latest"
    ]);

    match provider.raw_request::<_, serde_json::Value>("eth_call".into(), params).await {
        Ok(res) => println!("Simulation SUCCESS: {:?}", res),
        Err(e) => {
            println!("Simulation FAILED: Transaction would revert.");
            println!("Error info: {:?}", e);
        }
    }

    Ok(())
}

pub async fn execute_swap(token_address_str: &str) -> Result<()> {
    let rpc_url = env::var("BSC_RPC_URL")?;
    let private_key = env::var("PRIVATE_KEY")?;
    let buy_amount_bnb = env::var("BUY_AMOUNT_BNB")?.parse::<f64>()?;
    let router_address_str = env::var("PANCAKE_ROUTER")?;
    let router_address = Address::from_str(&router_address_str)?;
    let token_address = Address::from_str(token_address_str)?;

    let signer: PrivateKeySigner = private_key.parse()?;
    let wallet = EthereumWallet::from(signer.clone());

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_builtin(&rpc_url)
        .await?;

    let funds = U256::from((buy_amount_bnb * 1e18) as u64);
    
    println!("Preparing raw swap transaction for {} BNB of token {}", buy_amount_bnb, token_address);

    let router = FourMemeRouter::new(router_address, provider.clone());
    let call = router.buyTokenAMAP(token_address, funds, U256::ZERO);
    let calldata = call.calldata().to_vec();

    // 1. Get Nonce
    let nonce = provider.get_transaction_count(signer.address()).await?;

    // 2. Get Gas Price
    let gas_price = 80_000_000_000u128; // Fixed 80 Gwei
    
    // 3. Construct a raw Legacy Transaction (simplest for BSC)
    let mut tx = TxLegacy {
        chain_id: Some(56), // BSC Mainnet
        nonce,
        gas_price,
        gas_limit: 500_000,
        to: TxKind::Call(router_address),
        value: funds,
        input: Bytes::from(calldata),
    };

    println!("Signing raw transaction with {} Gwei gas price (Nonce: {}, Gas Limit: {})", gas_price as f64 / 1e9, nonce, tx.gas_limit);
    // 4. Sign the transaction using the PrivateKeySigner directly
    let signature = signer.sign_transaction(&mut tx).await?;
    
    // 5. Create the signed envelope
    let signed_tx = tx.into_signed(signature);
    let envelope = TxEnvelope::Legacy(signed_tx);
    let signed_tx_hex = format!("0x{}", hex::encode(envelope.encoded_2718()));

    println!("Broadcasting raw transaction... Hex length: {}", signed_tx_hex.len());

    // 6. Broadcast via raw JSON-RPC
    match provider.raw_request::<_, B256>("eth_sendRawTransaction".into(), vec![signed_tx_hex]).await {
        Ok(tx_hash) => println!("Transaction Sent! Hash: {:?}", tx_hash),
        Err(e) => eprintln!("Failed to broadcast: {:?}", e),
    }

    Ok(())
}
