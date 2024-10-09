## Info

Basic design of the transaction processor: Each transaction is streamed and processed as it arrives. The processor keeps in memory a record of all Deposits so that they can be referenced by disputes. (assumption is that only Deposits can be disputed)
 - For prod-ready code we'd need a way to purge stale entries from this data structure. An alternative solution would be to process the csv in two stage, first stage just to collect transactions reference by Disputes. But, this might not work well for a full streaming approach (where end of stream is not known).

Transactions amounts are represent by floats. This could be improved to use a decimal type suitable for financial transactions (to avoid rounding errors).

Tests could be improved by fuzzing or generally any automated test case generation. (ie, property based testing)