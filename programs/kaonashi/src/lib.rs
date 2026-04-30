use anchor_lang::prelude::*;

declare_id!("89JzYW3Jpk7xAn8Mbgx3JLfH5xmN67Tp23VEUam7Q7eE");

const MAX_PROPOSALS: usize = 5;
const MAX_PROPOSAL_NAME: usize = 32;
const NO_VOTE: u8 = u8::MAX;
const NO_FINAL_WINNER: u8 = u8::MAX;

#[program]
pub mod owner_project {
    use super::*;

    // 1. Cria uma nova votação.
    // A conta Ballot é a conta global: guarda chairperson, propostas e contagem total.
    pub fn initialize(ctx: Context<Initialize>, proposals_names: Vec<String>) -> Result<()> {
        // Não permite criar uma votação sem propostas.
        require!(!proposals_names.is_empty(), VotingError::NoProposals);

        // Só permite no máximo 5 propostas.
        require!(
            proposals_names.len() <= MAX_PROPOSALS,
            VotingError::TooManyProposals
        );

        let ballot = &mut ctx.accounts.ballot; // vamos alterar os dados de ballot

        ballot.chairperson = ctx.accounts.chairperson.key(); // endereço de quem criou a votação
        ballot.proposal_count = proposals_names.len() as u8; // quantas propostas existem
        ballot.proposals = Vec::new(); // cria uma lista vazia onde os nomes são adicionados no for
        ballot.tally = [0; MAX_PROPOSALS]; // [0, 0, 0, 0, 0]; ninguém votou
        ballot.final_winner_index = NO_FINAL_WINNER; // ainda não há vencedor final

        for name in proposals_names {
            // Cada nome de proposta pode ter no máximo 32 bytes.
            require!(
                name.as_bytes().len() <= MAX_PROPOSAL_NAME,
                VotingError::ProposalNameTooLong
            );

            // Os nomes das propostas ficam guardados on-chain.
            // Exemplo: proposals = ["wine", "beer", "water"]
            ballot.proposals.push(name);
        }

        Ok(())
    }

    // 2. O chairperson regista/autoriza um voter.
    // Isto cria uma conta PDA única para este voter nesta votação.
    pub fn register_voter(ctx: Context<RegisterVoter>, _voter_address: Pubkey) -> Result<()> {
        // _voter_address não é usado diretamente aqui dentro,
        // mas é usado nas seeds da PDA no Context RegisterVoter.
        let voter_record = &mut ctx.accounts.voter_record;

        voter_record.can_vote = true; // este voter está autorizado a votar
        voter_record.has_voted = false; // ainda não votou
        voter_record.vote = NO_VOTE; // ainda não existe voto guardado

        Ok(())
    }

    // 3. Um voter autorizado vota numa proposta.
    // A contagem global é atualizada imediatamente no ballot.tally.
    pub fn cast_vote(ctx: Context<CastVote>, proposal_index: u8) -> Result<()> {
        //  propostas e a contagem total dos votos.
        let ballot = &mut ctx.accounts.ballot;

        // Conta PDA individual deste voter.
        // Aqui está guardado se pode votar, se já votou e em que proposta votou.
        let voter_record = &mut ctx.accounts.voter_record;

        require!(voter_record.can_vote, VotingError::NotAllowedToVote); //// Verifica se este voter foi autorizado pelo chairperson.
        require!(!voter_record.has_voted, VotingError::AlreadyVoted); // Impede double voting.

        //[0, 1 ,2] votar no 3 não é valido
        require!(
            proposal_index < ballot.proposal_count,
            VotingError::InvalidProposal
        );

        // Converte o índice para usize,
        // porque arrays em Rust são indexados com usize.
        let index = proposal_index as usize;

        // Verifica que ainda não chegou ao máximo de u64.
        // Isto evita overflow antes de fazer += 1.
        require!(ballot.tally[index] < u64::MAX, VotingError::VoteOverflow);

        // Atualiza a conta individual do voter.
        voter_record.has_voted = true; // já votou
        voter_record.vote = proposal_index; // guarda o índice da proposta escolhida

        // Atualiza a contagem global da proposta escolhida.
        // Exemplo: se index = 1, então tally[1] aumenta em 1.
        ballot.tally[index] += 1;

        Ok(())
    }

    // 4. Em caso de empate, só o chairperson pode escolher o vencedor final.
    // O chairperson não vota normalmente só desempata
    pub fn resolve_tie(ctx: Context<ResolveTie>, winning_index: u8) -> Result<()> {
        let ballot = &mut ctx.accounts.ballot;

        // Verifica se a proposta escolhida existe.
        require!(
            winning_index < ballot.proposal_count,
            VotingError::InvalidProposal
        );

        // tally = [2, 2, 0, 0, 0]
        // active_tallies = [2, 2, 0]
        let active_tallies = &ballot.tally[..ballot.proposal_count as usize];

        // Procura o maior número de votos dentro de active_tallies.

        let max_votes = match active_tallies.iter().copied().max() {
            Some(value) => value, //encontrou o maior número de votos
            None => return Err(VotingError::NoProposals.into()), //não havia propostas para analisar
        };

        // Conta quantas propostas têm o número máximo de votos para verificar empate
        let tied_count = active_tallies
            .iter()
            .filter(|votes| **votes == max_votes)
            .count();

        // Verifica se há mesmo empate.
        require!(tied_count > 1, VotingError::NoTieToResolve);

        // tally = [3, 3, 1]
        // Pode escolher 0 ou 1.
        require!(
            ballot.tally[winning_index as usize] == max_votes,
            VotingError::WinnerMustBeTied
        );

        // Guarda o índice da proposta escolhida como vencedora final.
        // Isto não altera a contagem dos votos, apenas resolve o empate.
        ballot.final_winner_index = winning_index;

        // Mensagem para os logs da transação.
        msg!(
            "Chairperson resolved the winner to index: {}",
            winning_index
        );

        Ok(())
    }
}

// Accounts

#[account]
pub struct Ballot {
    pub chairperson: Pubkey,         // chairperson
    pub proposals: Vec<String>,      // ["wine", "beer", "water"]
    pub tally: [u64; MAX_PROPOSALS], // tally = [0, 0, 0, 0, 0]; a posição do tally corresponde ao índice da proposta
    pub proposal_count: u8,          // número real de propostas
    pub final_winner_index: u8, // vencedor final em caso de empate; NO_FINAL_WINNER se ainda não existir
}

impl Ballot {
    pub const SPACE: usize = 8 // discriminator Anchor
        + 32 // chairperson Pubkey
        + 4 + MAX_PROPOSALS * (4 + MAX_PROPOSAL_NAME) // Vec<String>: len + cada String
        + 8 * MAX_PROPOSALS // tally [u64; 5]
        + 1 // proposal_count
        + 1; // final_winner_index
}

#[account]
pub struct VoterRecord {
    pub can_vote: bool,  // indica se o voter foi autorizado pelo chairperson
    pub has_voted: bool, // indica se o voter já votou
    pub vote: u8,        // depois do voto, guarda o índice da proposta escolhida
}

impl VoterRecord {
    pub const SPACE: usize = 8 // discriminator Anchor
        + 1 // can_vote
        + 1 // has_voted
        + 1; // vote
}

// Contexts

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = chairperson, space = Ballot::SPACE)]
    pub ballot: Account<'info, Ballot>,

    #[account(mut)]
    pub chairperson: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(voter_address: Pubkey)]
pub struct RegisterVoter<'info> {
    #[account(mut)]
    pub chairperson: Signer<'info>,

    // Só o chairperson guardado dentro da Ballot pode registar voters.
    #[account(mut, has_one = chairperson)]
    pub ballot: Account<'info, Ballot>,

    // Conta PDA única para este voter nesta votação.
    // Isto faz com que o mesmo voter tenha uma conta diferente em cada eleição.
    #[account(
        init,
        payer = chairperson,
        space = VoterRecord::SPACE,
        seeds = [b"voter", ballot.key().as_ref(), voter_address.as_ref()],
        bump
    )]
    pub voter_record: Account<'info, VoterRecord>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CastVote<'info> {
    #[account(mut)]
    pub voter: Signer<'info>,

    #[account(mut)]
    pub ballot: Account<'info, Ballot>,

    // No voto, a PDA é derivada com o endereço de quem está a assinar.
    // Assim, o voter só consegue usar o seu próprio VoterRecord.
    #[account(
        mut,
        seeds = [b"voter", ballot.key().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub voter_record: Account<'info, VoterRecord>,
}

#[derive(Accounts)]
pub struct ResolveTie<'info> {
    pub chairperson: Signer<'info>,

    // Só o chairperson guardado na Ballot pode resolver o empate.
    #[account(mut, has_one = chairperson)]
    pub ballot: Account<'info, Ballot>,
}

// Errors
#[error_code]
pub enum VotingError {
    #[msg("No proposals provided.")]
    NoProposals,

    #[msg("Maximum 5 proposals allowed.")]
    TooManyProposals,

    #[msg("Proposal name is too long.")]
    ProposalNameTooLong,

    #[msg("Voter is not authorized to vote.")]
    NotAllowedToVote,

    #[msg("Voter has already cast a vote.")]
    AlreadyVoted,

    #[msg("Selected proposal index is out of bounds.")]
    InvalidProposal,

    #[msg("Vote counter overflow.")]
    VoteOverflow,

    #[msg("There is no tie to resolve.")]
    NoTieToResolve,

    #[msg("The chosen winner must be one of the tied proposals.")]
    WinnerMustBeTied,
}
