//
//  VectorF.swift
//  KeyPointCenter
//
//  Created by ZhaoTianyu on 2024/9/5.
//

import Foundation
import Accelerate

public typealias VectorA = [Float]

struct VectorF {
    static func add(_ a: VectorA, _ b: VectorA) -> VectorA {
        var result = VectorA(repeating: 0.0, count: a.count)
        vDSP_vadd(a, 1, b, 1, &result, 1, vDSP_Length(a.count))
        return result
    }
    
    static func sub(_ a: VectorA, _ b: VectorA) -> VectorA {
        var result = VectorA(repeating: 0.0, count: a.count)
        vDSP_vsub(a, 1, b, 1, &result, 1, vDSP_Length(a.count))
        return result
    }
    
    static func mul(_ a: VectorA, _ b: VectorA) -> VectorA {
        var result = VectorA(repeating: 0.0, count: a.count)
        vDSP_vmul(a, 1, b, 1, &result, 1, vDSP_Length(a.count))
        return result
    }
    
    static func mul(_ matrix: MatrixA, _ vector: VectorA) -> VectorA {
        let rowCount = matrix.count
        let flattenedMatrix = matrix.flatMap { $0 }
        var result = VectorA(repeating: 0.0, count: vector.count)
        let vectorCount = vDSP_Length(vector.count)
        vDSP_mmul(flattenedMatrix, 1, vector, 1, &result, 1, vectorCount, 1, vDSP_Length(rowCount))
        return result
    }
    
    static func div(_ a: VectorA, _ b: VectorA) -> VectorA {
        var result = VectorA(repeating: 0.0, count: a.count)
        vDSP_vdiv(a, 1, b, 1, &result, 1, vDSP_Length(a.count))
        return result
    }
}
