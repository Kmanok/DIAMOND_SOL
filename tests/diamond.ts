import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Diamond } from "../target/types/diamond";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY, Keypair } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, createMint, createAssociatedTokenAccount, mintTo } from '@solana/spl-token';
import { assert } from "chai";

describe("diamond", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Diamond as Program<Diamond>;
  
  // Test accounts
  let tokenState: PublicKey;
  let blacklist: PublicKey;
  let mint: PublicKey;
  let vault: PublicKey;
  let multisig: Keypair;
  let userTokenAccount: PublicKey;
  let userPaymentAccount: PublicKey;
  let mockPythPriceFeed: Keypair;

  // Test constants
  const INITIAL_SUPPLY = new anchor.BN("8000000000000000"); // 8_000_000 * 10^9
  const MAX_SUPPLY = new anchor.BN("100000000000000000"); // 100_000_000 * 10^9
  const TOKEN_PRICE_USDT = new anchor.BN("1000000"); // 1_000_000
  const TOKEN_PRICE_USDC = new anchor.BN("800000"); // 800_000

  before(async () => {
    // Create mock Pyth price feed account
    mockPythPriceFeed = anchor.web3.Keypair.generate();
    const rent = await provider.connection.getMinimumBalanceForRentExemption(3000);
    const createAccountIx = SystemProgram.createAccount({
      fromPubkey: provider.wallet.publicKey,
      newAccountPubkey: mockPythPriceFeed.publicKey,
      lamports: rent,
      space: 3000,
      programId: new PublicKey("FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH")
    });

    const transaction = new anchor.web3.Transaction().add(createAccountIx);
    
    // Исправляем передачу signers
    await provider.sendAndConfirm(
      transaction,
      [mockPythPriceFeed]  // Убираем provider.wallet.payer, так как он уже включен в provider
    );

    // Initialize test accounts
    [tokenState] = await PublicKey.findProgramAddress(
      [Buffer.from("token_state_v2")],
      program.programId
    );

    [blacklist] = await PublicKey.findProgramAddress(
      [Buffer.from("blacklist_v2")],
      program.programId
    );

    // Create mint account
    mint = await createMint(
      provider.connection,
      provider.wallet,
      provider.wallet.publicKey,
      null,
      9
    );
    
    // Create vault account
    vault = await createAssociatedTokenAccount(
      provider.connection,
      provider.wallet.payer,
      mint,
      provider.wallet.publicKey
    );
    
    // Create multisig account
    multisig = anchor.web3.Keypair.generate();
  });

  it("Initializes the token state", async () => {
    // Создаем Keypair для каждого владельца
    const multisigKeypairs = [
      anchor.web3.Keypair.generate(),
      anchor.web3.Keypair.generate(),
      anchor.web3.Keypair.generate(),
      anchor.web3.Keypair.generate(),
      anchor.web3.Keypair.generate(),
    ];

    // Получаем массив PublicKey из Keypair
    const multisigOwners = multisigKeypairs.map(kp => kp.publicKey);

    try {
      await program.methods
        .initialize(multisigOwners, new anchor.BN("3"))
        .accounts({
          payer: provider.wallet.publicKey,
          tokenState,
          mint,
          blacklist,
          multisig: multisig.publicKey,
          vault,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .signers([provider.wallet.payer, multisig, ...multisigKeypairs])
        .rpc();

      // Verify token state
      const tokenStateAccount = await program.account.tokenState.fetch(tokenState);
      assert.ok(tokenStateAccount.totalSupply.eq(INITIAL_SUPPLY));
      assert.ok(tokenStateAccount.maxSupply.eq(MAX_SUPPLY));
      assert.ok(tokenStateAccount.isPaused === false);
    } catch (error) {
      console.error("Initialization error:", error);
      throw error;
    }
  });

  it("Mints tokens with USDT payment", async () => {
    // Create test user
    const user = anchor.web3.Keypair.generate();
    
    // Create mock USDT mint and accounts
    const usdtMint = await createMint(
      provider.connection,
      provider.wallet,
      provider.wallet.publicKey,
      null,
      6
    );

    // Create user USDT account
    userPaymentAccount = await createAssociatedTokenAccount(
      provider.connection,
      provider.wallet,
      usdtMint,
      user.publicKey
    );

    // Create user token account
    userTokenAccount = await createAssociatedTokenAccount(
      provider.connection,
      provider.wallet,
      mint,
      user.publicKey
    );

    // Mint some USDT to user
    await mintTo(
      provider.connection,
      provider.wallet,
      usdtMint,
      userPaymentAccount,
      provider.wallet,
      1000000000
    );

    const amount = new anchor.BN("1000000000000"); // 1000 tokens

    try {
      await program.methods
        .mintByUser(amount)
        .accounts({
          user: user.publicKey,
          tokenState,
          mint,
          paymentToken: usdtMint,
          userPaymentAccount,
          userTokenAccount,
          vault,
          blacklist,
          solPriceFeed: mockPythPriceFeed.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Verify mint
      const balance = await provider.connection.getTokenAccountBalance(userTokenAccount);
      assert.ok(new anchor.BN(balance.value.amount).eq(amount));
    } catch (error) {
      console.error("Mint error:", error);
      throw error;
    }
  });

  it("Pauses and unpauses token operations", async () => {
    try {
      // Pause
      await program.methods
        .pause()
        .accounts({
          authority: provider.wallet.publicKey,
          tokenState,
          multisig: multisig.publicKey,
        })
        .signers([provider.wallet.payer, multisig])
        .rpc();

      // Verify pause
      let tokenStateAccount = await program.account.tokenState.fetch(tokenState);
      assert.ok(tokenStateAccount.isPaused === true);

      // Unpause
      await program.methods
        .unpause()
        .accounts({
          authority: provider.wallet.publicKey,
          tokenState,
          multisig: multisig.publicKey,
        })
        .signers([provider.wallet.payer, multisig])
        .rpc();

      // Verify unpause
      tokenStateAccount = await program.account.tokenState.fetch(tokenState);
      assert.ok(tokenStateAccount.isPaused === false);
    } catch (error) {
      console.error("Pause/Unpause error:", error);
      throw error;
    }
  });

  it("Updates blacklist", async () => {
    const addressToBlacklist = anchor.web3.Keypair.generate().publicKey;

    try {
      // Add to blacklist
      await program.methods
        .addToBlacklist(addressToBlacklist)
        .accounts({
          authority: provider.wallet.publicKey,
          tokenState,
          blacklist,
          multisig: multisig.publicKey,
        })
        .signers([provider.wallet.payer, multisig])
        .rpc();

      // Verify blacklist
      let blacklistAccount = await program.account.blacklist.fetch(blacklist);
      assert.ok(blacklistAccount.addresses.some(addr => addr.equals(addressToBlacklist)));

      // Remove from blacklist
      await program.methods
        .removeFromBlacklist(addressToBlacklist)
        .accounts({
          authority: provider.wallet.publicKey,
          tokenState,
          blacklist,
          multisig: multisig.publicKey,
        })
        .signers([provider.wallet.payer, multisig])
        .rpc();

      // Verify removal
      blacklistAccount = await program.account.blacklist.fetch(blacklist);
      assert.ok(!blacklistAccount.addresses.some(addr => addr.equals(addressToBlacklist)));
    } catch (error) {
      console.error("Blacklist update error:", error);
      throw error;
    }
  });
});