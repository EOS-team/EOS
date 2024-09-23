//
//  KalmanFilter.swift
//  MetaScripts
//
//  Created by ZhaoTianyu on 2024/7/10.
//

import Foundation
import simd

class KalmanFilter {
    // Fields
    private var effectiveCount = 0
    private var kalmanFilter: DiscreteKalmanFilter<ConstantVelocity3DModel, simd_float3>
    private let startCount = 50

    // Methods
    init(timeInterval: Float, noise: Float) {
        let initialState = ConstantVelocity3DModel()
        initialState.Position = simd_float3(0, 0, 0)
        initialState.Velocity = simd_float3(0, 0, 0)
        
        let processNoise = ConstantVelocity3DModel.getProcessNoise(noise)
        let measurementVectorDimension = 3
        let controlVectorDimension = 0
        let stateConvertFunc = ConstantVelocity3DModel.toArray
        let stateConvertBackFunc = ConstantVelocity3DModel.fromArray
        let measurementConvertFunc = Testc.test930 ?? Testc.b30
        
        kalmanFilter = DiscreteKalmanFilter(
            initialState: initialState,
            initialStateError: processNoise,
            measurementVectorDimension: measurementVectorDimension,
            controlVectorDimension: controlVectorDimension,
            stateConvertFunc: stateConvertFunc,
            stateConvertBackFunc: stateConvertBackFunc,
            measurementConvertFunc: measurementConvertFunc
        )

        kalmanFilter.processNoise = ConstantVelocity3DModel.getProcessNoise(noise, timeInterval: timeInterval)
        kalmanFilter.measurementNoise = MatrixF.diagonal(columnCount: kalmanFilter.measurementVectorDimension!, value: 1)
        kalmanFilter.measurementMatrix = ConstantVelocity3DModel.getPositionMeasurementMatrix()
        kalmanFilter.transitionMatrix = ConstantVelocity3DModel.getTransitionMatrix(timeInterval: timeInterval)
        kalmanFilter.predict()
    }

    func correct(kp: simd_float3) {
        kalmanFilter.correct(measurement: kp)
    }

    func correctAndPredict(kp: simd_float3) -> simd_float3 {
        kalmanFilter.correct(measurement: kp)
        kalmanFilter.predict()
        if effectiveCount < 50 {
            effectiveCount += 1
            return kp
        }
        return getPosition()
    }

    func getPosition() -> simd_float3 {
        return kalmanFilter.State!.Position
    }

    func predict() {
        kalmanFilter.predict()
    }

    func updateFilterParameter(timeInterval: Float, noise: Float) {
        kalmanFilter.processNoise = ConstantVelocity3DModel.getProcessNoise(noise, timeInterval: timeInterval)
        kalmanFilter.transitionMatrix = ConstantVelocity3DModel.getTransitionMatrix(timeInterval: timeInterval)
        effectiveCount = 0
    }
}

// Nested Types
class Testc {
    typealias MeasurementConvertFunc = (simd_float3) -> VectorA
    
    // TODO: C#中貌似有序列化读取的操作，否则此值永远为空，不知道怎么处理
    static var test930: MeasurementConvertFunc?

    // Methods
    static var b30: MeasurementConvertFunc = {
        return [$0.x, $0.y, $0.z]
    }
}
