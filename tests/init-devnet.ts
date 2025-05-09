// @ts-nocheck
import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY, Keypair } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, getAssociatedTokenAddress, setAuthority, createAccount, getMinimumBalanceForRentExemptAccount, MintLayout, TokenAccountLayout, createInitializeAccountInstruction } from "@solana/spl-token";
import { Diamond } from "../target/types/diamond";

const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

const program = anchor.workspace.Diamond as anchor.Program<Diamond>;

// Функция для создания обычного SPL TokenAccount с PDA owner
async function createTokenAccountWithPDA(connection, payer, mint, owner) {
  const tokenAccount = Keypair.generate();
  const lamports = await getMinimumBalanceForRentExemptAccount(connection);
  const transaction = new anchor.web3.Transaction();

  transaction.add(
    anchor.web3.SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: tokenAccount.publicKey,
      space: 165,
      lamports,
      programId: TOKEN_PROGRAM_ID,
    }),
    createInitializeAccountInstruction(
      tokenAccount.publicKey,
      mint,
      owner,
      TOKEN_PROGRAM_ID
    )
  );

  await anchor.web3.sendAndConfirmTransaction(
    connection,
    transaction,
    [payer, tokenAccount]
  );

  return tokenAccount.publicKey;
}

async function main() {
  try {
    // 1. Создаем 5 multisig владельцев (включая наш wallet)
    const multisigKeypairs = [
      Keypair.generate(),
      Keypair.generate(),
      Keypair.generate(),
      Keypair.generate(),
      Keypair.generate(),
    ];

    // Получаем массив PublicKey из Keypair
    const multisigOwners = multisigKeypairs.map(kp => kp.publicKey);

    console.log("\nMultisig Owners:");
    multisigOwners.forEach((owner, index) => {
      console.log(`Owner ${index + 1}:`, owner.toBase58());
    });

    // 2. PDA для token_state, blacklist и vault
    const [tokenState] = await PublicKey.findProgramAddress(
      [Buffer.from("token_state_v2")],
      program.programId
    );
    const [blacklist] = await PublicKey.findProgramAddress(
      [Buffer.from("blacklist_v2")],
      program.programId
    );
    const [vaultPda] = await PublicKey.findProgramAddress(
      [Buffer.from("vault_v2")],
      program.programId
    );

    console.log("\nPDAs:");
    console.log("Token State:", tokenState.toBase58());
    console.log("Blacklist:", blacklist.toBase58());
    console.log("Vault:", vaultPda.toBase58());

    // 3. Создаём mint с authority = wallet
    const mint = await createMint(
      provider.connection,
      provider.wallet.payer,
      provider.wallet.publicKey, // authority сначала wallet
      null,
      9
    );

    // 3.1. Передаём authority PDA tokenState
    // await provider.connection.requestAirdrop(tokenState, 1e7); // Убрано, чтобы не было ошибки лимита
    await setAuthority(
      provider.connection,
      provider.wallet.payer,
      mint,
      provider.wallet.publicKey,
      'MintTokens',
      tokenState
    );

    console.log("\nMint created:", mint.toBase58());

    // 3.2. Создаём vault вручную (SPL TokenAccount с PDA owner)
    const vault = await createTokenAccountWithPDA(
      provider.connection,
      provider.wallet.payer,
      mint,
      tokenState // owner = PDA tokenState
    );

    // 4. Создаём multisig аккаунт
    const multisig = Keypair.generate();
    console.log("Multisig account:", multisig.publicKey.toBase58());

    // 5. Вызов initialize
    console.log("\nInitializing token state...");

    const tx = await program.methods
      .initialize(multisigOwners, new anchor.BN(3))  // Порог 3 из 5
      .accounts({
        payer: provider.wallet.publicKey,
        tokenState: tokenState,
        mint: mint,
        blacklist: blacklist,
        multisig: multisig.publicKey,
        vault: vault,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([provider.wallet.payer])
      .rpc();

    console.log("\nИнициализация успешно завершена!");
    console.log("------------------------");
    console.log("Token State:", tokenState.toBase58());
    console.log("Mint:", mint.toBase58());
    console.log("Vault:", vault.toBase58());
    console.log("Multisig:", multisig.publicKey.toBase58());
    
    console.log("\nMultisig Owners (save these for future use):");
    multisigKeypairs.forEach((kp, index) => {
      console.log(`Owner ${index + 1} Public Key:`, kp.publicKey.toBase58());
      console.log(`Owner ${index + 1} Private Key:`, Buffer.from(kp.secretKey).toString('base64'));
    });
    
  } catch (error) {
    console.error("\nError during initialization:");
    console.error(error);
  }
}

main(); 