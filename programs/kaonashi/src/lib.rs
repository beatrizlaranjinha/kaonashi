use anchor_lang::prelude::*;

declare_id!("89JzYW3Jpk7xAn8Mbgx3JLfH5xmN67Tp23VEUam7Q7eE");

//equivalente ao contrato na solidity, at least i hope
#[program]
pub mod owner_project {

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let voting_state = &mut ctx.accounts.voting_state; //vai buscar a conta onde vamos buscar dados
        voting_state.owner = *ctx.accounts.user.key; // key=address
        voting_state.value = 0;
        Ok(())
    }

    pub fn set_value(ctx: Context<OnlyOwner>, value: u64) -> Result<()> {
        //recebe um numero value , só pode ser chamada pelo dono
        //Atualiza o valor
        let voting_state = &mut ctx.accounts.voting_state;
        voting_state.value = value;
        Ok(())
    }

    pub fn change_owner(ctx: Context<OnlyOwner>, new_owner: Pubkey) -> Result<()> {
        let voting_state = &mut ctx.accounts.voting_state;
        //Muda o dono
        voting_state.owner = new_owner;
        Ok(())
    }
}

//equivalente as variaveis de contrato i think
#[account]
pub struct VotingState {
    pub owner: Pubkey,
    pub value: u64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 32 + 8)]
    //criar uma nova conta com init, sapce memoria necessaria
    pub voting_state: Account<'info, VotingState>, //onde ficam guardados os dados
    #[account(mut)]
    pub user: Signer<'info>, //quem chama assina
    pub system_program: Program<'info, System>, // programa inter da solana para criar contas
}

#[derive(Accounts)]
pub struct OnlyOwner<'info> {
    #[account(mut, has_one = owner)]
    pub voting_state: Account<'info, VotingState>,
    pub owner: Signer<'info>,
}
