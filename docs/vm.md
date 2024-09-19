

### 1. **Compilation and Bytecode Generation**
   - When we compile a smart contract, it gets transformed into bytecode. This bytecode is what will be deployed onto the blockchain. Along with the bytecode, the **ABI (Application Binary Interface)** is generated, which defines how you can interact with the contract (e.g., what functions can be called and what parameters are needed).

### 2. **Contract Deployment**
   - When we deploy a smart contract, the bytecode is included in a transaction that is sent to the blockchain. However, additional information such as the **sender's address (deployer)** and possibly some initial parameters are included in this transaction. The contract address is actually **determined by the blockchain** based on the deployer's address and the nonce (number of transactions sent by the deployer).
   - Once deployed, the smart contract exists as a **self-contained entity** on the blockchain, with its own address and state.

### 3. **Contracts as Actors**
   - The contract can be considered an "actor" on the blockchain. It holds its **own state** and the **bytecode** for its execution logic. The contract's state can include variables like balances, ownership, and other data.
   - Importantly, the contract has no private key or ability to initiate actions on its own—it only responds to external transactions or calls. These calls are made by external accounts (EOAs) or other contracts.

### 4. **Invoking Contract Methods (Transactions)**
   - When a client wants to interact with a smart contract (e.g., call a function), a **transaction** is created. This transaction contains:
   - The **address** of the contract being called.
   - The **function signature** and any **parameters** for the function.
   - The **gas limit** and **gas price**.
   - The **sender's address** and any **value** (if sending cryptocurrency, e.g., Ether on Ethereum).
   
   This transaction is then broadcast to the blockchain.

### 5. **Transaction Execution by Validators**
   - When a smart contract function is invoked, the transaction gets processed by the **validators (or miners)**. These validators execute the contract's bytecode in the blockchain's **virtual machine (VM)**.
   - The execution happens in a **deterministic** way on every validator node, meaning that all nodes must execute the same code and arrive at the same result. This ensures **consensus**.
   
   **Clarification**: Not all contract function calls require a full transaction. Some calls are "read-only" (view/pure functions), which means they do not alter the contract state. These can be executed **locally** on a node without involving validators. Only state-changing calls (e.g., transferring tokens, modifying contract data) require transactions and gas fees.


### 6. **Execution on Validator Hardware**
   - When a state-changing transaction is sent, the contract's bytecode is executed on the **validator nodes' hardware** within the VM environment. Each node runs the same bytecode, ensuring that all nodes reach the same output and update the contract's state consistently.
   - Validators (or miners) execute the transaction, consume **gas** for every operation, and modify the state accordingly.
   - Once the transaction is processed, the **new state** of the contract is written to the blockchain.

### Additional Points:
   - **Gas Mechanism**: Every operation performed by a smart contract consumes a certain amount of **gas**, which is paid by the sender of the transaction. The gas ensures that the network is compensated for the computational resources used, and also prevents infinite loops or denial-of-service attacks.
   - **Consensus**: The result of contract execution is agreed upon by the network's consensus mechanism (e.g., Proof of Work, Proof of Stake). Once the consensus is reached, the state is updated across the entire network.


When we refer to **contract bytecode**, we're talking about the **compiled form** of the smart contract. This bytecode is essentially a machine-readable version of the smart contract, which is deployed on the blockchain when the contract is created. It includes the logic and instructions that the contract will execute, but it is not in human-readable form like the original source code (e.g., Solidity).

### Breakdown of What Happens When a Function is Invoked!?

1. **Contract Bytecode on the Chain**:
   - Once a smart contract is deployed, its **bytecode** is stored at the contract's address on the blockchain. This bytecode remains there permanently, and any interaction with the contract refers back to this bytecode.
   - The bytecode contains all the logic of the contract: its functions, fallback functions, and any storage structures.
   - The blockchain nodes store this bytecode and reference it whenever an interaction occurs with the contract.

2. **How a Function Call Is Encoded in a Transaction**:
   - When a client (user or another contract) wants to call a function on a smart contract, the **transaction data** field includes the following:
     - **Function selector**: A 4-byte identifier derived from the function's **signature**. The signature is a hash of the function name and its parameters (e.g., `transfer(address,uint256)`).
     - **Encoded arguments**: The parameters (arguments) that are passed to the function, encoded in a format understood by the contract (using Ethereum's ABI encoding, for example).

   Example:
   - Let's say you are calling `transfer(address recipient, uint256 amount)` in a smart contract.
   - The first 4 bytes of the transaction data will be the **function selector**, which is the first 4 bytes of the keccak256 hash of the function signature: `transfer(address,uint256)`.
   - The rest of the transaction data will be the encoded values of `recipient` and `amount`.

3. **How the Virtual Machine (VM) Executes the Call**:
   - When the transaction is received by a validator node, it needs to **execute the function call**. This happens in the blockchain's **virtual machine**.
   - The VM looks at the **contract address** specified in the transaction and retrieves the **contract's bytecode** from storage (the blockchain).
   - The transaction's **data field** (which contains the function selector and encoded parameters) is passed into the VM.

4. **Matching the Function Call to the Bytecode**:
   - Inside the contract bytecode, each function has its own entry point. The VM uses the **function selector** from the transaction to identify which function is being called.
     - When a contract is compiled, the bytecode includes a **jump table** or a **dispatcher** logic that knows how to route incoming function calls based on their selectors.
     - The VM takes the first 4 bytes of the transaction's data (the function selector), looks it up in the contract's bytecode, and **jumps** to the corresponding section of the bytecode where that function is defined.

5. **Executing the Function**:
   - Once the correct function has been located within the bytecode, the VM begins executing the instructions defined for that function, using the provided parameters.
   - During execution, the VM may modify the contract's **storage** (e.g., updating balances, variables) or transfer cryptocurrency, depending on the logic of the function.
   - All of this is executed in a deterministic manner, meaning every validator node will perform the exact same computation to ensure consensus.

### Example of How This Works in Practice:
Let's take a simplified scenario where you're interacting with a deployed contract to call the `transfer(address,uint256)` function.

1. **Step 1: Create a Transaction**
   - You create a transaction that specifies:
     - The **contract address**.
     - The **data field** containing the **function selector** for `transfer(address,uint256)` (the first 4 bytes of the hash of the function signature).
     - The **encoded arguments** (e.g., the recipient's address and the amount to transfer).

2. **Step 2: Validator Receives the Transaction**
   - The validator receives the transaction and sees that it is directed at a specific **contract address**. The contract bytecode is already stored at this address on the blockchain.
   
3. **Step 3: VM Execution**
   - The VM loads the contract's bytecode and examines the **function selector** in the transaction's data.
   - It matches this selector to a specific section in the contract's bytecode that corresponds to the `transfer` function.

4. **Step 4: Function Execution**
   - The VM starts executing the bytecode for the `transfer` function, using the **arguments** provided in the transaction data (i.e., recipient and amount).
   - If the function involves modifying balances or state variables, these changes are performed and then written to the blockchain as part of the **contract's state**.

### Summary of Key Concepts:
- **Contract Bytecode**: The compiled code of the smart contract stored at the contract's address. It contains all the logic of the contract's functions.
- **Function Call**: When a contract function is invoked, the transaction contains a **function selector** and encoded arguments.
- **VM Execution**: The virtual machine retrieves the contract's bytecode and uses the **function selector** to identify which part of the bytecode to execute.
- **Jump Table/Dispatcher**: Part of the bytecode that helps route the transaction to the correct function in the contract.

Let's walk through the entire process from compiling a smart contract to invoking a function on it, step by step. This will cover everything from compiling the contract code to interacting with it on the blockchain.

```
1) compile contract using compiler 
2) create an actor contract struct contains bytecode, owner, ... on the chain 
3) assign an address to the bytecode of actor contract
4) function call of the contract in form of a hashed tx will be sent to the chain 
5) all validators at the same receive it, they fetch the contract bytecode using its address
6) all validators at the same time try to match the contract function signature in the tx against the actor contract bytecode
7) all validators at the same execute the tx in vm 
8) actor contract state gets mutated in the entire chain
```

### 1. **Writing the Smart Contract**

The first step is writing the smart contract code. This is typically done in a high-level language that is suitable for the blockchain platform you're working on. For example:
- **Ethereum**: Solidity or Vyper
- **Solana**: Rust
- **Binance Smart Chain**: Solidity

Here's an example of a simple Solidity contract:

```solidity
pragma solidity ^0.8.0;

contract SimpleStorage {
    uint256 storedData;

    function set(uint256 data) public {
        storedData = data;
    }

    function get() public view returns (uint256) {
        return storedData;
    }
}
```

### 2. **Compiling the Contract**

Once the contract code is written, it needs to be **compiled** into **bytecode** and an **ABI (Application Binary Interface)**. The compiler translates the human-readable source code (like Solidity) into **bytecode**, which is understood by the blockchain's **virtual machine**.

#### Compilation Output:
- **Bytecode**: The machine code that the blockchain can execute.
- **ABI**: A JSON structure that defines how to interact with the contract, including details about its functions, arguments, and return types.

For example, after compiling the `SimpleStorage` contract, you might get the following:
- **Bytecode**: `0x608060405234801561001057600080fd5b50610120806100206000396000f3fe...`
- **ABI**: 
```json
[
    {
        "inputs": [
            {
                "internalType": "uint256",
                "name": "data",
                "type": "uint256"
            }
        ],
        "name": "set",
        "outputs": [],
        "stateMutability": "nonpayable",
        "type": "function"
    },
    {
        "inputs": [],
        "name": "get",
        "outputs": [
            {
                "internalType": "uint256",
                "name": "",
                "type": "uint256"
            }
        ],
        "stateMutability": "view",
        "type": "function"
    }
]
```

### 3. **Deploying the Contract**

Now that you have the **bytecode** and **ABI**, you can deploy the contract to the blockchain. The deployment process involves sending a **transaction** to the blockchain that contains the bytecode and possibly some initial parameters. Here's how it works:

#### Steps:
1. **Create a Transaction**:
   - A transaction is created to deploy the contract. This transaction includes:
     - **Bytecode**: The compiled contract code.
     - **Sender's Address**: The account deploying the contract (this becomes the deployer or owner).
     - **Gas Limit**: The maximum amount of gas the deployer is willing to spend for deployment.

2. **Broadcast the Transaction**:
   - The transaction is broadcast to the network, and validators (or miners, depending on the chain) will pick it up.
   - If the transaction is successful, a **contract address** is assigned to the deployed contract. This address is deterministically created using the **deployer's address** and the **transaction nonce**.

3. **Deployment Confirmation**:
   - After the contract is deployed, it gets an **address on the blockchain**. This address is where the contract is stored, and it's where users or other contracts can interact with it.
   
   Example: `0x1234...5678` could be the contract's new address on the blockchain.

### 4. **Interacting with the Deployed Contract**

Once the contract is deployed, it can be interacted with by invoking its functions. Interactions with smart contracts can either:
- **Read data (view)**: Call a function that reads from the contract's storage without modifying it.
- **Write data (state-changing)**: Call a function that modifies the contract's state, such as changing stored values, transferring tokens, etc.

#### Example of Function Call:
Let's say you want to call the `set` function to store the number `42` in the `SimpleStorage` contract. The process involves:

#### Steps:
1. **Create a Transaction** (for state-changing function calls):
   - To invoke a contract function that modifies the state, you must create a **transaction**. This includes:
     - **Contract Address**: The address of the deployed contract.
     - **Function Selector**: The first 4 bytes of the keccak256 hash of the function signature (e.g., `set(uint256)`).
     - **Encoded Parameters**: The parameters for the function, encoded using the contract's ABI.
     - **Gas Limit & Gas Price**: The amount of gas you're willing to pay for the transaction.
     - **Sender's Address**: The address of the account making the transaction.

   For `set(uint256)`, you encode `42` as the parameter:
   - **Function Selector** for `set(uint256)`: `0x60fe47b1`
   - **Encoded Data**: `000000000000000000000000000000000000000000000000000000000000002a` (42 in hex)

   The **transaction data** would look something like:
   ```text
   0x60fe47b1000000000000000000000000000000000000000000000000000000000000002a
   ```

2. **Send the Transaction**:
   - This transaction is then sent to the blockchain. Validators will pick up the transaction and start executing it using the contract's **bytecode** that is stored at the contract's address.

3. **VM Execution**:
   - The validators (nodes running the Ethereum Virtual Machine or another blockchain's VM) will execute the bytecode stored at the contract address. They match the **function selector** with the corresponding function in the bytecode and begin execution using the **parameters** provided in the transaction.

4. **State Change**:
   - As the function executes, the VM modifies the contract's state. In the case of `set(uint256)`, the value `42` will be stored in the contract's storage variable `storedData`.

5. **Transaction Finalization**:
   - Once the transaction is processed, it is added to a block, and the state changes (like the new value `42` in `storedData`) are finalized across the network.

### 5. **Reading Data from the Contract (View Functions)**

For functions that don't modify the contract state (like the `get()` function), you can **call** them without creating a full transaction. These are "view" or "pure" functions that only read data and don't require a gas fee.

#### Steps:
1. **Call the Contract Function**:
   - You can create a **read-only call** to the contract. In this case, there's no need to pay gas since no state change is involved.
   
2. **VM Executes Locally**:
   - The contract's bytecode is executed locally by a node, and the result is returned. In the case of `get()`, the value stored in `storedData` (which we set to `42`) will be returned.

3. **Return Value**:
   - The result of the function (`42` in this case) is returned to the user or client application, and no state changes occur.

### Complete Flow Summary

1. **Contract Compilation**:
   - Source code is compiled into bytecode and ABI.

2. **Contract Deployment**:
   - A transaction with the contract's bytecode is sent to the blockchain, and the contract is assigned an address.

3. **Function Invocation (State-Changing)**:
   - To invoke a contract's function (e.g., `set(uint256)`), a transaction is created with encoded function call data, sent to the network, and executed by validators, updating the contract's state.

4. **Function Invocation (Read-Only)**:
   - Read-only function calls (e.g., `get()`) are executed locally by the node and do not involve a transaction or gas fees.

In the context of blockchain smart contracts, it is **not accurate** to say that contract actors (i.e., smart contracts) communicate directly through traditional RPC (Remote Procedure Call) protocols like **gRPC** or **Cap'n Proto (capnpc)**. Instead, blockchain communication and interaction between smart contracts follow a different paradigm based on transactions and blockchain infrastructure. Here's why:

### Key Differences Between Smart Contract Communication and RPC

1. **Smart Contracts on Blockchain**:
   - **Smart contracts** are self-contained programs that exist on the blockchain and only execute when they are **invoked** through transactions.
   - They do not initiate actions or "calls" on their own, unlike traditional client-server RPC models. Instead, they are **passive** and only respond to **external transactions** initiated by users or other contracts.
   - Communication between smart contracts happens **within the blockchain environment** (e.g., Ethereum Virtual Machine for Ethereum-based blockchains) and is executed as part of the same transaction, **not through external RPC protocols**.

2. **Transaction-Based Communication**:
   - When a smart contract invokes another smart contract (i.e., a function from another contract), it happens **within the blockchain**, using the virtual machine's execution environment. This is fundamentally different from external systems communicating over RPC.
   - Contract-to-contract communication happens **synchronously** as part of a single transaction. The invoking contract includes the address of the contract being called and passes encoded data (using ABI encoding) as part of the transaction.
   - This communication happens **on-chain** and does not leave the blockchain ecosystem.

3. **No Network Calls Involved**:
   - Unlike **gRPC** or **Cap'n Proto**, which are used for inter-service communication across a network (e.g., between microservices in a distributed system), smart contracts do not rely on external network protocols. Everything happens inside the blockchain, and there is no need for external protocol translation.
   - Calls between contracts do not involve standard networking layers (e.g., HTTP/2, TCP) or serialization formats used in gRPC or Cap'n Proto.

### How Smart Contract Communication Actually Works:

Here's how smart contracts typically communicate with one another on a blockchain like Ethereum:

1. **Contract A Calls Contract B**:
   - Contract A wants to invoke a function on Contract B. This can happen within the same transaction.
   - Contract A knows the address of Contract B and prepares a function call (using the ABI to encode the function selector and parameters).
   
2. **Execution in the VM**:
   - The VM handles the execution. It routes the call to Contract B, checks its bytecode, and executes the appropriate function based on the provided data.
   - Any state changes or computations happen inside the VM, and the result of Contract B's function call is returned to Contract A.
   
3. **Gas Consumption**:
   - All these interactions consume **gas**, and the costs are part of the single transaction. The contract making the call (Contract A) must ensure it has provided sufficient gas for the transaction, which includes both the computation in Contract A and the interaction with Contract B.

4. **Atomicity and Reentrancy**:
   - All contract interactions within a transaction are **atomic**, meaning that if any step fails (e.g., the call to Contract B fails), the entire transaction is reverted.
   - This ensures consistency across the blockchain but introduces considerations like **reentrancy attacks**, which need to be mitigated in contract design.

### External Communication and Oracles

While smart contracts do not use RPC to communicate with each other on-chain, they can indirectly interact with **off-chain systems** using **oracles**. Oracles act as bridges between off-chain data and smart contracts, providing external information such as price feeds, weather data, or events from real-world systems.

- **Oracles** typically communicate with off-chain systems using traditional networking protocols (sometimes even gRPC or other RPC systems).
- However, when oracles relay data back to the smart contract, they do so via on-chain **transactions**—again, not through RPC calls directly.

### When is RPC Used?

RPC protocols like **gRPC** or **Cap'n Proto** are used in blockchain contexts, but their role is more in the context of how **users** or **clients** communicate with blockchain nodes, rather than how contracts communicate with each other on-chain.

- **JSON-RPC** is a commonly used protocol for communication between clients and Ethereum nodes. For example, when a user or an application wants to interact with the blockchain (e.g., querying the blockchain or sending a transaction), they use JSON-RPC over HTTP or WebSocket to communicate with a node.
- **gRPC** or **Cap'n Proto** might be used by some blockchain infrastructure services, but this would be for things like node-to-node communication, off-chain services, or layer 2 solutions.

### Summary:

- Smart contracts communicate with each other through **on-chain transactions** executed within the **blockchain's virtual machine**, not through external RPC protocols like gRPC or Cap'n Proto.
- All communication between contracts is **transactional** and handled within the blockchain environment, using the **contract address**, **function selector**, and **encoded parameters**.
- RPC protocols like **JSON-RPC** are used for **client-to-node** communication (i.e., how external users interact with the blockchain), but **not for contract-to-contract** interactions on-chain.

If you're looking for how **off-chain systems** or **clients** interact with the blockchain, then RPC (e.g., JSON-RPC) is involved, but **contract-to-contract communication** stays entirely within the blockchain's execution environment.

since the contract compiled bytecode is stored on the chain we can use abi/idl of the contract for RPC calls which helps us to see what methods and types are inside the contract in order to create tx object and send it to the chain through RPC calls which contains the contract methods.