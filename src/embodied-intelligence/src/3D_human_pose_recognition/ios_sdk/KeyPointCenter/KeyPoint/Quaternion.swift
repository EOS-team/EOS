//
//  Quaternion.swift
//  MetaScripts
//
//  Created by ZhaoTianyu on 2024/7/11.
//

import Foundation
import simd

public typealias Quaternion = simd_quatf

extension Quaternion {
    func description() -> String {
        return "[\(imag.x), \(imag.y), \(imag.z), \(real)]"
    }
    
    static var zero: Quaternion {
        return Quaternion(vector: .zero)
    }
    
    static func lookRotation(forward: Vector3, up: Vector3) -> Quaternion {
        let forward = normalize(forward)
        let upwards = normalize(up)
        
        let right = cross(upwards, forward)
        let up = cross(forward, right)
        
        let m00 = right.x
        let m01 = right.y
        let m02 = right.z
        let m10 = up.x
        let m11 = up.y
        let m12 = up.z
        let m20 = forward.x
        let m21 = forward.y
        let m22 = forward.z
        
        let trace = m00 + m11 + m22
        var temp = simd_quatf()
        
        if trace > 0 {
            let s = 0.5 / sqrt(trace + 1.0)
            temp.real = 0.25 / s
            temp.imag = simd_float3((m12 - m21) * s, (m20 - m02) * s, (m01 - m10) * s)
        } else {
            if m00 > m11 && m00 > m22 {
                let s = 2.0 * sqrt(1.0 + m00 - m11 - m22)
                temp.real = (m12 - m21) / s
                temp.imag = simd_float3(0.25 * s, (m01 + m10) / s, (m20 + m02) / s)
            } else if m11 > m22 {
                let s = 2.0 * sqrt(1.0 + m11 - m00 - m22)
                temp.real = (m20 - m02) / s
                temp.imag = simd_float3((m01 + m10) / s, 0.25 * s, (m12 + m21) / s)
            } else {
                let s = 2.0 * sqrt(1.0 + m22 - m00 - m11)
                temp.real = (m01 - m10) / s
                temp.imag = simd_float3((m20 + m02) / s, (m12 + m21) / s, 0.25 * s)
            }
        }
        
        return temp
    }

    static func slerp(from: Quaternion, to: Quaternion, t: Float) -> Quaternion {
        return simd_slerp(from, to, t)
    }

    static func lerp(_ start: Quaternion, _ end: Quaternion, _ t: Float) -> Quaternion {
        let tClamped = max(0, min(1, t))
        return simd_slerp(start, end, tClamped)
    }
}
