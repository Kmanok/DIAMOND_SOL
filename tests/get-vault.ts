import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

async function main() {
    try {
        // Setup provider
        const provider = anchor.AnchorProvider.env();
        anchor.setProvider(provider);

        // Get program from workspace
        const program = anchor.workspace.Diamond;

        // 1. Get token_state PDA
        const [token_state] = await PublicKey.findProgramAddress(
            [Buffer.from("token_state")],
            program.programId
        );

        console.log("Fetching token state account...");
        console.log("Token State PDA:", token_state.toBase58());

        // 2. Fetch token state account
        const tokenStateAccount = await program.account.tokenState.fetch(token_state);
        
        // 3. Get account info to check size
        const accountInfo = await provider.connection.getAccountInfo(token_state);
        const accountSize = accountInfo?.data.length || 0;

        // 4. Print all relevant information
        console.log("\nToken State Information:");
        console.log("------------------------");
        console.log("Account size:", accountSize, "bytes");
        console.log("Vault address:", tokenStateAccount.vault.toBase58());
        console.log("Mint address:", tokenStateAccount.mint.toBase58());
        console.log("Total supply:", tokenStateAccount.totalSupply.toString());
        console.log("Max supply:", tokenStateAccount.maxSupply.toString());
        console.log("Is paused:", tokenStateAccount.isPaused);
        console.log("Authority:", tokenStateAccount.authority.toBase58());
        console.log("Multisig:", tokenStateAccount.multisig.toBase58());

    } catch (error) {
        console.error("\nError occurred:");
        console.error("----------------");
        console.error(error);
        
        if (error.message.includes("Account does not exist")) {
            console.error("\nPossible solutions:");
            console.error("1. Make sure you've initialized the token state with 'token_state' seed");
            console.error("2. Check if you're using the correct program ID");
            console.error("3. Verify your wallet has enough SOL for the transaction");
        }

        if (error.logs) {
            console.log('Transaction logs:', error.logs);
        }
    }
}

main(); 