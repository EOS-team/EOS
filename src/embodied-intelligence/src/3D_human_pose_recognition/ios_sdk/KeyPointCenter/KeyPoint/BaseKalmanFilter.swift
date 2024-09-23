//
//  BaseKalmanFilter.swift
//  MetaScripts
//
//  Created by ZhaoTianyu on 2024/7/8.
//

import Foundation

public class BaseKalmanFilter<TState, TMeasurement> {
    typealias MeasurementConvertFunc = (TMeasurement) -> [Float]
    typealias StateConvertBackFunc = ([Float]) -> TState
    typealias StateConvertFunc = (TState) -> [Float]
    
    var measurementConvertFunc: MeasurementConvertFunc?
    var state: [Float]?
    var stateConvertBackFunc: StateConvertBackFunc?
    var stateConvertFunc: StateConvertFunc?
    var controlMatrix: [[Float]]?
    var controlVectorDimension: Int?
    var estimateCovariance: [[Float]]?
    var kalmanGain: [[Float]]?
    var measurementMatrix: [[Float]]?
    var measurementNoise: [[Float]]?
    var measurementVectorDimension: Int?
    var processNoise: [[Float]]?
    var residualCovariance: [[Float]]?
    var residualCovarianceInv: [[Float]]?
    var State: TState? {
        get {
            guard let state = state else {
                return nil
            }
            return stateConvertBackFunc?(state)
        }
    }
    var stateVectorDimension: Int?
    var transitionMatrix: [[Float]]?
    
    required init(
        initialState: TState,
        initialStateError: [[Float]],
        measurementVectorDimension: Int,
        controlVectorDimension: Int,
        stateConvertFunc: @escaping StateConvertFunc,
        stateConvertBackFunc: @escaping StateConvertBackFunc,
        measurementConvertFunc: @escaping MeasurementConvertFunc
    ) {
        let numArray = stateConvertFunc(initialState)
        self.stateVectorDimension = numArray.count
        self.measurementVectorDimension = measurementVectorDimension;
        self.controlVectorDimension = controlVectorDimension;
        self.state = numArray;
        self.estimateCovariance = initialStateError;
        self.stateConvertFunc = stateConvertFunc
        self.stateConvertBackFunc = stateConvertBackFunc
        self.measurementConvertFunc = measurementConvertFunc
    }
    
    func calculateDelta(measurement: [Float]) -> [Float] {
        guard let measurementMatrix = measurementMatrix, let state = state else {
            return []
        }
        
        let b = VectorF.mul(measurementMatrix, state)
        return VectorF.sub(measurement, b)
    }
    func correct(measurement: TMeasurement) {
        guard let measurement = measurementConvertFunc?(measurement) else {
            return
        }
        correctInternal(measurement: measurement)
    }
    func correctInternal(measurement: [Float]) {
        
    }
    
    func predict() {
        predict(controlVector: nil)
    }
    func predict(controlVector: [Float]?) {
        predictInternal(controlVector: controlVector)
    }
    func predictInternal(controlVector: [Float]?) {
        
    }
}
