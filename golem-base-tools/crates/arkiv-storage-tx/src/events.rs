alloy_sol_types::sol! {
    contract ArkivABI {
        event ArkivEntityCreated(
            uint256 indexed entityKey,
            address indexed owner,
            uint256 expirationBlock,
            uint256 cost,
        );

        event ArkivEntityUpdated(
            uint256 indexed entityKey,
            address indexed owner,
            uint256 oldExpirationBlock,
            uint256 newExpirationBlock,
            uint256 cost,
        );

        event ArkivEntityDeleted(
            uint256 indexed entityKey,
            uint256 indexed owner,
        );

        event ArkivEntityBTLExtended(
            uint256 indexed entityKey,
            address indexed owner,
            uint256 oldExpirationBlock,
            uint256 newExpirationBlock,
            uint256 cost
        );

        event ArkivEntityOwnerChanged(
            uint256 indexed entityKey,
            uint256 indexed oldOwner,
            uint256 indexed newOwner,
        );

        event ArkivEntityExpired(
            uint256 indexed entityKey,
            uint256 indexed owner,
        );
    }
}
