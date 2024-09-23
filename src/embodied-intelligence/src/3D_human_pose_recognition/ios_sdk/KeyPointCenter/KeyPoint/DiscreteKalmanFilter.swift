//
//  DiscreteKalmanFilter.swift
//  MetaScripts
//
//  Created by ZhaoTianyu on 2024/7/8.
//

import Foundation

class DiscreteKalmanFilter<TState, TMeasurement>: BaseKalmanFilter<TState, TMeasurement> {
    required init(
        initialState: TState,
        initialStateError: [[Float]],
        measurementVectorDimension: Int,
        controlVectorDimension: Int,
        stateConvertFunc: @escaping StateConvertFunc,
        stateConvertBackFunc: @escaping StateConvertBackFunc,
        measurementConvertFunc: @escaping MeasurementConvertFunc
    ) {
        super.init(
            initialState: initialState,
            initialStateError: initialStateError,
            measurementVectorDimension: measurementVectorDimension,
            controlVectorDimension: controlVectorDimension,
            stateConvertFunc: stateConvertFunc,
            stateConvertBackFunc: stateConvertBackFunc,
            measurementConvertFunc: measurementConvertFunc
        )
    }
    
    func correct(innovationVector: [Float]) {
        if innovationVector.count != measurementVectorDimension {
            fatalError("PredicitionError error vector (innovation vector) must have the same length as measurement.")
        }
        
        // TODO: 验证、优化强制解包逻辑
        let b = VectorF.mul(kalmanGain!, innovationVector)
        state = VectorF.add(state!, b)
    }
    
    // MARK: - From Base
    override func correctInternal(measurement: [Float]) {
        let innovationVector = calculateDelta(measurement: measurement)
        correct(innovationVector: innovationVector)
    }
    
    override func predictInternal(controlVector: [Float]?) {
        // TODO: 验证、优化强制解包逻辑
        
        state = VectorF.mul(transitionMatrix!, state!)

        if let controlVector = controlVector {
            state = VectorF.add(
                state!,
                VectorF.mul(controlMatrix!, controlVector)
            )
        }
        
        estimateCovariance = MatrixF.add(
            MatrixF.mul(
                MatrixF.mul(transitionMatrix!, estimateCovariance!),
                MatrixF.transpose(transitionMatrix!)
            ),
            processNoise!
        )
        
        let b = MatrixF.transpose(measurementMatrix!)
        residualCovariance = MatrixF.add(
            MatrixF.mul(
                MatrixF.mul(measurementMatrix!, estimateCovariance!),
                b
            ),
            measurementNoise!
        )

        residualCovarianceInv = MatrixF.inverse(residualCovariance!)
        kalmanGain = MatrixF.mul(
            MatrixF.mul(estimateCovariance!, b),
            residualCovarianceInv!
        )
    }
}
