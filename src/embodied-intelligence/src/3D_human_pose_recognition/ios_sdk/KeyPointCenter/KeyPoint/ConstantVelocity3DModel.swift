//
//  ConstantVelocity3DModel.swift
//  MetaScripts
//
//  Created by ZhaoTianyu on 2024/7/8.
//

import Foundation
import simd

class ConstantVelocity3DModel: NSObject, NSCopying {
    typealias StateConvertFunc = (ConstantVelocity3DModel) -> [Float]
    typealias StateConvertBackFunc = (VectorA) -> ConstantVelocity3DModel
    
    // Fields
    static let Dimension = 6
    var Position = simd_float3()
    var Velocity = simd_float3()

    // Methods
    func copy(with zone: NSZone? = nil) -> Any {
        let copy = ConstantVelocity3DModel()
        copy.Position = self.Position
        copy.Velocity = self.Velocity
        return copy
    }
    
    static var fromArray: StateConvertBackFunc = {
        let model = ConstantVelocity3DModel()
        model.Position = simd_float3($0[0], $0[2], $0[4])
        model.Velocity = simd_float3($0[1], $0[3], $0[5])
        return model
    }

    static func getPositionMeasurementMatrix() -> [[Float]] {
        return [
            [1, 0, 0, 0, 0, 0],
            [0, 0, 1, 0, 0, 0],
            [0, 0, 0, 0, 1, 0]
        ]
    }

    static func getProcessNoise(_ accelerationNoise: Float, timeInterval: Float = 1) -> [[Float]] {
        var matrix = [[Float]](repeating: [Float](repeating: 0, count: 3), count: 6)
        matrix[0][0] = (timeInterval * timeInterval) / 2
        matrix[1][0] = timeInterval
        matrix[2][1] = (timeInterval * timeInterval) / 2
        matrix[3][1] = timeInterval
        matrix[4][2] = (timeInterval * timeInterval) / 2
        matrix[5][2] = timeInterval

        let b = MatrixF.diagonal(columnCount: matrix[0].count, value: accelerationNoise)
        return MatrixF.mul(MatrixF.mul(matrix, b), MatrixF.transpose(matrix))
    }

    static func getTransitionMatrix(timeInterval: Float = 1) -> [[Float]] {
        let num = timeInterval
        var matrix = [[Float]](repeating: [Float](repeating: 0, count: 6), count: 6)
        matrix[0][0] = 1
        matrix[1][1] = 1
        matrix[2][2] = 1
        matrix[3][3] = 1
        matrix[4][4] = 1
        matrix[5][5] = 1
        matrix[0][1] = num
        matrix[2][3] = num
        matrix[4][5] = num
        return matrix
    }
    
    static var toArray: StateConvertFunc = { modelState in
        return [modelState.Position.x, modelState.Velocity.x, modelState.Position.y, modelState.Velocity.y, modelState.Position.z, modelState.Velocity.z]
    }
}


