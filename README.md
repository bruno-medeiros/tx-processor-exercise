## Info

Basic design of the transaction processor: Each transaction is streamed and processed as it arrives.
The processor keeps in memory a record of all transactions so that they can be referenced by disputes.

Transactions amounts are represent by floats. This could be improved to use a decimal type suitable for financial transactions (to avoid rounding errors).

Assumptions: 
 * Disputed transactions can only be Deposits or Withdrawals