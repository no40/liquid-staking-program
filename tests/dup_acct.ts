import * as anchor from "@project-serum/anchor";
import { Wallet } from "@project-serum/anchor/dist/cjs/provider"
import Keypair from "@project-serum/anchor/dist/cjs/provider";
//import { ToyToken } from "../target/types/toy_token";
import { Connection, Signer, PublicKey, SystemProgram, Transaction, TransactionSignature, ConfirmOptions, Commitment, SendOptions } from "@solana/web3.js";

describe("dup_acct", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.MarinadeFinance;
    const admin_authority = anchor.web3.Keypair.generate();
    const acct1keys = anchor.web3.Keypair.generate();

    it("can airdrop to acct", async () => {

        // account is a namespace, it looks in the IDL provided ( by Anchor ) and picks out a (case insensitive)
        // account flagged with the #[account] macro that matches the name we expect, specified by the discriminator.
        // create_instruction on the account namespace might do what I need for creating an arbitrary acct 
        console.debug("howdy")

    const airdrop = async (toKey: PublicKey, sol: number) => {
        const ix = SystemProgram.transfer({
            fromPubkey: provider.wallet.publicKey,
            toPubkey: toKey,
            lamports: 1000000000 * sol, 
        });
        const tx = new Transaction().add(ix);
        const blockhash = await provider.connection.getLatestBlockhash('finalized');
        tx.recentBlockhash = blockhash.blockhash;
        tx.feePayer = provider.wallet.publicKey;
        await provider.wallet.signTransaction(tx);
        await provider.sendAndConfirm(tx); 
    }

    await airdrop(acct1keys.publicKey, 600);
    //@ts-ignore
    //let acct1Data = await program.accounts.tokenAccount.fetch(acct1keys.publicKey);
    //console.debug(acct1Data.amount.toNumber());
    //expect(acct1Data.amount.toNumber()).toEqual(-1);

    //class Fee {
    //basis_points: number = 100;
    //}
    //class ConfigMarinadeParams {
    //rewards_fee: Fee = new Fee();
    //slot_for_stake_ 

    //}
    //await program.methods
    //.config_marinade()
    //.accounts({ admin_authority: admin_authority })
    //.signers([admin_authority])
    //.rpc({ commitment: "confirmed" });

    //await program.methods
    //.transfer(new anchor.BN(5))
    //.accounts({ payer: acct1keys.publicKey, payee: acct.publicKey })
    //.signers([acct1keys])
    //.rpc({ commitment: "confirmed" });

    //let acct1keysData = await program.account.tokenAccount.fetch(acct.publicKey);
    //expect(acct1keysData.amount.toNumber()).toEqual(10);
    });

    it("can set up stake accts", async () => {
        //@ts-ignore
        await program.methods.initialize({validator_manager_authority: provider.wallet.publicKey, admin_authority: provider.wallet.publicKey }).accounts({
            creator_authority: provider.wallet.publicKey,
            // no state

        }).signers([provider.wallet])
        .rpc({ commitment: "confirmed" })
        //}
        //await program.methods
        //.config_marinade()
        //.accounts({ admin_authority: admin_authority })
        //.signers([admin_authority])
        //.rpc({ commitment: "confirmed" });

})
});
