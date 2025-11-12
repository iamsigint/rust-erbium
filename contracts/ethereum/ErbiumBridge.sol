// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/**
 * @title ErbiumBridge
 * @dev Cross-chain bridge contract for transferring assets between Erbium and Ethereum
 * @notice This contract handles locking/unlocking of ERC20 tokens and native ETH
 */
contract ErbiumBridge {
    // Events
    event TokensLocked(
        address indexed user,
        address indexed token,
        uint256 amount,
        bytes32 indexed erbiumAddress,
        uint256 nonce
    );

    event TokensUnlocked(
        address indexed user,
        address indexed token,
        uint256 amount,
        bytes32 erbiumTxHash
    );

    event BridgePaused(address indexed pauser);
    event BridgeUnpaused(address indexed pauser);
    event ValidatorAdded(address indexed validator);
    event ValidatorRemoved(address indexed validator);

    // State variables
    mapping(address => bool) public validators;
    mapping(bytes32 => bool) public processedTransactions;
    mapping(address => uint256) public lockedBalances;

    address[] public validatorList;
    uint256 public requiredSignatures;
    uint256 public nonce;
    bool public paused;

    // Constants
    uint256 public constant MAX_LOCK_AMOUNT = 1000000 * 10**18; // 1M tokens max per tx
    uint256 public constant MIN_LOCK_AMOUNT = 10**6; // 0.000001 tokens min
    bytes32 public constant ETH_ADDRESS = 0x0000000000000000000000000000000000000000;

    // Modifiers
    modifier onlyValidator() {
        require(validators[msg.sender], "Not a validator");
        _;
    }

    modifier whenNotPaused() {
        require(!paused, "Bridge is paused");
        _;
    }

    modifier whenPaused() {
        require(paused, "Bridge is not paused");
        _;
    }

    modifier validAmount(uint256 amount) {
        require(amount >= MIN_LOCK_AMOUNT, "Amount too small");
        require(amount <= MAX_LOCK_AMOUNT, "Amount too large");
        _;
    }

    modifier validAddress(bytes32 erbiumAddress) {
        require(erbiumAddress != bytes32(0), "Invalid Erbium address");
        _;
    }

    /**
     * @dev Constructor
     * @param _validators Initial validator addresses
     * @param _requiredSignatures Number of signatures required for operations
     */
    constructor(address[] memory _validators, uint256 _requiredSignatures) {
        require(_validators.length > 0, "At least one validator required");
        require(_requiredSignatures > 0, "Required signatures must be > 0");
        require(_requiredSignatures <= _validators.length, "Too many required signatures");

        for (uint256 i = 0; i < _validators.length; i++) {
            require(_validators[i] != address(0), "Invalid validator address");
            require(!validators[_validators[i]], "Duplicate validator");

            validators[_validators[i]] = true;
            validatorList.push(_validators[i]);

            emit ValidatorAdded(_validators[i]);
        }

        requiredSignatures = _requiredSignatures;
        paused = false;
        nonce = 0;
    }

    /**
     * @dev Lock ERC20 tokens for cross-chain transfer
     * @param token Token contract address (address(0) for ETH)
     * @param amount Amount to lock
     * @param erbiumAddress Destination address on Erbium chain
     */
    function lockTokens(
        address token,
        uint256 amount,
        bytes32 erbiumAddress
    )
        external
        payable
        whenNotPaused
        validAmount(amount)
        validAddress(erbiumAddress)
    {
        if (token == address(0)) {
            // ETH transfer
            require(msg.value == amount, "Incorrect ETH amount");
        } else {
            // ERC20 transfer
            require(msg.value == 0, "ETH not accepted for ERC20 transfers");

            // Transfer tokens from user to bridge
            (bool success,) = token.call(
                abi.encodeWithSignature("transferFrom(address,address,uint256)", msg.sender, address(this), amount)
            );
            require(success, "Token transfer failed");

            lockedBalances[token] += amount;
        }

        nonce++;
        emit TokensLocked(msg.sender, token, amount, erbiumAddress, nonce);
    }

    /**
     * @dev Unlock tokens after cross-chain transfer (validator only)
     * @param user Recipient address
     * @param token Token contract address
     * @param amount Amount to unlock
     * @param erbiumTxHash Transaction hash from Erbium chain
     * @param signatures Validator signatures
     */
    function unlockTokens(
        address user,
        address token,
        uint256 amount,
        bytes32 erbiumTxHash,
        bytes[] calldata signatures
    )
        external
        onlyValidator
        whenNotPaused
        validAmount(amount)
    {
        require(!processedTransactions[erbiumTxHash], "Transaction already processed");
        require(signatures.length >= requiredSignatures, "Insufficient signatures");

        // Verify signatures
        bytes32 messageHash = keccak256(abi.encodePacked(user, token, amount, erbiumTxHash));
        bytes32 ethSignedMessageHash = keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash));

        address[] memory signers = new address[](signatures.length);
        for (uint256 i = 0; i < signatures.length; i++) {
            address signer = recoverSigner(ethSignedMessageHash, signatures[i]);
            require(validators[signer], "Invalid signature");
            signers[i] = signer;
        }

        // Check for duplicate signatures
        for (uint256 i = 0; i < signers.length - 1; i++) {
            for (uint256 j = i + 1; j < signers.length; j++) {
                require(signers[i] != signers[j], "Duplicate signature");
            }
        }

        // Mark transaction as processed
        processedTransactions[erbiumTxHash] = true;

        if (token == address(0)) {
            // Unlock ETH
            (bool success,) = user.call{value: amount}("");
            require(success, "ETH transfer failed");
        } else {
            // Unlock ERC20 tokens
            require(lockedBalances[token] >= amount, "Insufficient locked balance");
            lockedBalances[token] -= amount;

            (bool success,) = token.call(
                abi.encodeWithSignature("transfer(address,uint256)", user, amount)
            );
            require(success, "Token transfer failed");
        }

        emit TokensUnlocked(user, token, amount, erbiumTxHash);
    }

    /**
     * @dev Add a new validator (admin function)
     * @param validator New validator address
     */
    function addValidator(address validator) external onlyValidator {
        require(validator != address(0), "Invalid validator address");
        require(!validators[validator], "Already a validator");

        validators[validator] = true;
        validatorList.push(validator);

        emit ValidatorAdded(validator);
    }

    /**
     * @dev Remove a validator (admin function)
     * @param validator Validator address to remove
     */
    function removeValidator(address validator) external onlyValidator {
        require(validators[validator], "Not a validator");
        require(validatorList.length > requiredSignatures, "Cannot remove last validator");

        validators[validator] = false;

        // Remove from validator list
        for (uint256 i = 0; i < validatorList.length; i++) {
            if (validatorList[i] == validator) {
                validatorList[i] = validatorList[validatorList.length - 1];
                validatorList.pop();
                break;
            }
        }

        emit ValidatorRemoved(validator);
    }

    /**
     * @dev Update required signatures
     * @param newRequiredSignatures New number of required signatures
     */
    function updateRequiredSignatures(uint256 newRequiredSignatures) external onlyValidator {
        require(newRequiredSignatures > 0, "Required signatures must be > 0");
        require(newRequiredSignatures <= validatorList.length, "Too many required signatures");

        requiredSignatures = newRequiredSignatures;
    }

    /**
     * @dev Pause the bridge (emergency stop)
     */
    function pause() external onlyValidator whenNotPaused {
        paused = true;
        emit BridgePaused(msg.sender);
    }

    /**
     * @dev Unpause the bridge
     */
    function unpause() external onlyValidator whenPaused {
        paused = false;
        emit BridgeUnpaused(msg.sender);
    }

    /**
     * @dev Get locked balance for a token
     * @param token Token address
     * @return Locked balance
     */
    function getLockedBalance(address token) external view returns (uint256) {
        return lockedBalances[token];
    }

    /**
     * @dev Get list of all validators
     * @return Array of validator addresses
     */
    function getValidators() external view returns (address[] memory) {
        return validatorList;
    }

    /**
     * @dev Check if transaction was processed
     * @param txHash Transaction hash
     * @return True if processed
     */
    function isTransactionProcessed(bytes32 txHash) external view returns (bool) {
        return processedTransactions[txHash];
    }

    /**
     * @dev Recover signer from signature
     * @param ethSignedMessageHash Hash of the signed message
     * @param signature Signature
     * @return Signer address
     */
    function recoverSigner(bytes32 ethSignedMessageHash, bytes memory signature) internal pure returns (address) {
        require(signature.length == 65, "Invalid signature length");

        bytes32 r;
        bytes32 s;
        uint8 v;

        assembly {
            r := mload(add(signature, 0x20))
            s := mload(add(signature, 0x40))
            v := byte(0, mload(add(signature, 0x60)))
        }

        return ecrecover(ethSignedMessageHash, v, r, s);
    }

    /**
     * @dev Fallback function to receive ETH
     */
    receive() external payable {}

    /**
     * @dev Emergency withdraw function (only in paused state)
     * @param token Token to withdraw
     * @param amount Amount to withdraw
     */
    function emergencyWithdraw(address token, uint256 amount) external onlyValidator whenPaused {
        if (token == address(0)) {
            (bool success,) = msg.sender.call{value: amount}("");
            require(success, "ETH transfer failed");
        } else {
            (bool success,) = token.call(
                abi.encodeWithSignature("transfer(address,uint256)", msg.sender, amount)
            );
            require(success, "Token transfer failed");
        }
    }
}
