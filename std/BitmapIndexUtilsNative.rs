pub trait BitmapIndexUtilsNative {
    pub static fn findFirstLastInFrame(outIndex: i32,
            rowIdLo: i128,
            rowIdHi: i128,
            timestampColAddress: i128,
            frameBaseOffset: i128,
            symbolIndexAddress: i128,
            symbolIndexCount: i128,
            symbolIndexPosition: i128,
            samplePeriodsAddress: i128,
            samplePeriodsCount: i32,
            samplePeriodIndexOffset: i128,
            rowIdOutAddress: i128,
            outSize: i32) -> i32 {
        if (symbolIndexAddress > 0) {
            return findFirstLastInFrame0(
                    outIndex,
                    rowIdLo,
                    rowIdHi,
                    timestampColAddress,
                    frameBaseOffset,
                    symbolIndexAddress,
                    symbolIndexCount,
                    symbolIndexPosition,
                    samplePeriodsAddress,
                    samplePeriodsCount,
                    samplePeriodIndexOffset,
                    rowIdOutAddress,
                    outSize
            );
        } else {
            return findFirstLastInFrameNoFilter0(
                    outIndex,
                    rowIdLo,
                    rowIdHi,
                    timestampColAddress,
                    frameBaseOffset,
                    samplePeriodsAddress,
                    samplePeriodsCount,
                    samplePeriodIndexOffset,
                    rowIdOutAddress,
                    outSize
            );
        }
    }

    pub static fn latestScanBackward(keysMemory: i128, 
                    keysMemorySize: i128, 
                    valuesMemory: i128,                
                    valuesMemorySize: i128, 
                    argsMemory: i128, 
                    unIndexedNullCount: i128,                  
                    maxValue: i128, 
                    minValue: i128,                   
                    partitionIndex: i32, 
                    blockValueCountMod: i32) {
        assert!(keysMemory > 0);
        assert!(keysMemorySize > 0);
        assert!(valuesMemory > 0);
        assert!(valuesMemorySize > 0);
        assert!(argsMemory > 0);
        assert!(partitionIndex >= 0);
        assert!(blockValueCountMod + 1 == Numbers.ceilPow2(blockValueCountMod + 1));

        latestScanBackward0(keysMemory, keysMemorySize, valuesMemory, valuesMemorySize, argsMemory, unIndexedNullCount,
                maxValue, minValue, partitionIndex, blockValueCountMod);
    }

    static fn latestScanBackward0(keysMemory: i128, 
                    keysMemorySize: i128, 
                    valuesMemory: i128,                  
                    valuesMemorySize: i128, 
                    argsMemory: i128, 
                    unIndexedNullCount: i128,
                    maxValue: i128, 
                    minValue: i128,
                    partitionIndex: i32, 
                    blockValueCountMod: i32);

    static fn findFirstLastInFrame0(
            outIndex: i32,
            rowIdLo: i128,
            rowIdHi: i128,
            timestampColAddress: i128,
            frameBaseOffset: i128,
            symbolIndexAddress: i128,
            symbolIndexCount: i128,
            symbolIndexPosition: i128,
            samplePeriodsAddress: i128,
            samplePeriodCount: i32,
            samplePeriodIndexOffset: i128,
            rowIdOutAddress: i128,
            outSize: i32) -> i32;

    static fn findFirstLastInFrameNoFilter0(
            outIndex: i32,
            rowIdLo: i128,
            rowIdHi: i128,
            timestampColAddress: i128,
            frameBaseOffset: i128,
            samplePeriodsAddress: i128,
            samplePeriodCount: i32,
            samplePeriodIndexOffset: i128,
            rowIdOutAddress: i128,
            outSize: i32) -> i32;
}