## Info

Basic design of the transaction processor: Each transaction is streamed and processed as it arrives. A futures Stream is used for more flexibility in different cases (network etc). 
  - The processor keeps in memory a record of all Deposits so that they can be referenced by disputes. Assumption here is that only Deposits can be disputed, based on how the spec is phrased: 
    - > "This means that the clients available funds should decrease by the amount disputed, their held funds should increase by the amount disputed"

 - Transactions amounts are represent by fastnum decimal type, not floats. fastnum is used instead of BigDecimal as it's faster and more memory efficient.

 - Tests could be improved by fuzzing or generally any automated test case generation. (ie, property based testing)

 - Made assumption that transactions can only Resolved/Chargedback once. (TODO: needs tests for this)


## AI use
 * AI was just used for minor edits: changing the processor from using Iterator to using Stream, change the amount type to fastnum and fix the tests compilation using .into(), asking question about how to print fastnum without trailing zeros, and that was about it.