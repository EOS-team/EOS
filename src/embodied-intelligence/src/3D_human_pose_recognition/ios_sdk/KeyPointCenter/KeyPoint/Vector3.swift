//
//  Vector3.swift
//  MetaScripts
//
//  Created by ZhaoTianyu on 2024/7/10.
//

import Foundation
import simd

public typealias Vector3 = simd_float3

extension Vector3 {
    func description() -> String {
        return "[\(x), \(y), \(z)]"
    }
    
    static var forward: Vector3 {
        return Vector3(0, 0, 1)
    }
    
    static var up: Vector3 {
        return Vector3(0, 1, 0)
    }

    static func Angle(_ v1: Vector3, _ v2: Vector3) -> Float {
        let dotProduct = simd_dot(v1, v2)
        let lengthsMultiplication = simd_length(v1) * simd_length(v2)
        let cosineOfAngle = dotProduct / lengthsMultiplication

        // acos 返回的角度是以弧度为单位的，需要转化为度
        let angleInRadians = acos(cosineOfAngle)
        let angleInDegrees = angleInRadians * (180.0 / Float.pi)
        
        return angleInDegrees
    }

    func distance(to: Vector3) -> Float {
        return simd_distance(self, to)
    }
}
