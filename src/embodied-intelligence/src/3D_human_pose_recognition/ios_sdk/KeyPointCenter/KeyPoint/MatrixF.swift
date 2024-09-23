//
//  MatrixF.swift
//  KeyPointCenter
//
//  Created by ZhaoTianyu on 2024/9/5.
//

import Foundation

public typealias MatrixA = [[Float]]

struct MatrixF {
    static func diagonal(columnCount: Int, value: Float) -> MatrixA {
        var matrix = MatrixA(repeating: [Float](repeating: 0, count: columnCount), count: columnCount)
        for i in 0..<columnCount {
            matrix[i][i] = value
        }
        return matrix
    }
    
//    static func add(_ a: MatrixA, _ b: MatrixA) -> MatrixA {
//        let rowCount = a.count
//        let columnCount = a[0].count
//        var result = Array(repeating: Array(repeating: Float(0.0), count: columnCount), count: rowCount)
//        for i in 0..<rowCount {
//            result.append(VectorF.add(a[i], b[i]))
//        }
//        return result
//    }
    
    static func add(_ a: MatrixA, _ b: MatrixA) -> MatrixA {
        if a.count != b.count || a[0].count != b[0].count {
            fatalError("Matrix dimensions must match")
        }
        
        let length = a.count
        let num2 = a[0].count
        var numArray = Array(repeating: Array(repeating: Float(0.0), count: num2), count: length)
        
        for i in 0..<num2 {
            for j in 0..<length {
                numArray[j][i] = a[j][i] + b[j][i]
            }
        }
        
        return numArray
    }
    
    static func sub(_ a: MatrixA, _ b: MatrixA) -> MatrixA {
        let rowCount = a.count
        let columnCount = a[0].count
        var result = Array(repeating: Array(repeating: Float(0.0), count: columnCount), count: rowCount)
        for i in 0..<rowCount {
            result.append(VectorF.sub(a[i], b[i]))
        }
        return result
    }
    
//    static func mul(_ a: MatrixA, _ b: MatrixA) -> MatrixA {
//        let rowCount = a.count
//        let columnCount = b[0].count
//        let commonDimension = a[0].count
//        var result = MatrixA(repeating: VectorA(repeating: 0, count: columnCount), count: rowCount)
//        for i in 0..<rowCount {
//            for j in 0..<columnCount {
//                for k in 0..<commonDimension {
//                    result[i][j] += a[i][k] * b[k][j]
//                }
//            }
//        }
//        return result
//    }
    static func mul(_ a: MatrixA, _ b: MatrixA) -> MatrixA {
        var result = Array(repeating: Array(repeating: Float(0.0), count: b[0].count), count: a.count)
        mul(a, b, result: &result)
        return result
    }
    
    static func mul(_ a: MatrixA, _ columnVector: [Float]) -> [Float] {
        let length = a.count
        if a[0].count != columnVector.count {
            fatalError("columnVector Vector must have the same length as columns in the matrix.")
        }
        var numArray = Array(repeating: Float(0.0), count: length)
        for i in 0..<length {
            for j in 0..<columnVector.count {
                numArray[i] += a[i][j] * columnVector[j]
            }
        }
        return numArray
    }
    
    static func mul(_ a: MatrixA, _ b: MatrixA, result: inout MatrixA) {
        let length = result.count
        let num2 = result[0].count
        var numArray = Array(repeating: Float(0.0), count: a[0].count)
        for i in 0..<num2 {
            for j in 0..<numArray.count {
                numArray[j] = b[j][i]
            }
            for k in 0..<length {
                var num6: Float = 0.0
                for m in 0..<numArray.count {
                    num6 += a[k][m] * numArray[m]
                }
                result[k][i] = num6
            }
        }
    }

    
    static func transpose(_ matrix: MatrixA) -> MatrixA {
        let rowCount = matrix.count
        let columnCount = matrix[0].count
        var result = MatrixA(repeating: VectorA(repeating: 0, count: rowCount), count: columnCount)
        
        for i in 0..<rowCount {
            for j in 0..<columnCount {
                result[j][i] = matrix[i][j]
            }
        }
        return result
    }
    
    static func inverse(_ matrix: MatrixA, _ inPlace: Bool = false) -> MatrixA? {
        let length = matrix.count
        let num2 = matrix[0].count
        if length != num2 {
            fatalError("Matrix must be square")
        }
        
        if length == 3 {
            let num3 = matrix[0][0]
            let num4 = matrix[0][1]
            let num5 = matrix[0][2]
            let num6 = matrix[1][0]
            let num7 = matrix[1][1]
            let num8 = matrix[1][2]
            let num9 = matrix[2][0]
            let num10 = matrix[2][1]
            let num11 = matrix[2][2]
            
            let num12 = ((num3 * ((num7 * num11) - (num8 * num10))) - (num4 * ((num6 * num11) - (num8 * num9)))) + (num5 * ((num6 * num10) - (num7 * num9)))
            
            if num12 == 0.0 {
                fatalError("Matrix is singular")
            }
            
            let num13 = 1.0 / num12
            var result = inPlace ? matrix : [[Float]](repeating: [Float](repeating: 0.0, count: 3), count: 3)
            
            result[0][0] = num13 * ((num7 * num11) - (num8 * num10))
            result[0][1] = num13 * ((num5 * num10) - (num4 * num11))
            result[0][2] = num13 * ((num4 * num8) - (num5 * num7))
            result[1][0] = num13 * ((num8 * num9) - (num6 * num11))
            result[1][1] = num13 * ((num3 * num11) - (num5 * num9))
            result[1][2] = num13 * ((num5 * num6) - (num3 * num8))
            result[2][0] = num13 * ((num6 * num10) - (num7 * num9))
            result[2][1] = num13 * ((num4 * num9) - (num3 * num10))
            result[2][2] = num13 * ((num3 * num7) - (num4 * num6))
            
            return result
        }
        
        if length != 2 {
            fatalError("Matrix size not supported")
        }
        
        let num14 = matrix[0][0]
        let num15 = matrix[0][1]
        let num16 = matrix[1][0]
        let num17 = matrix[1][1]
        let num18 = (num14 * num17) - (num15 * num16)
        
        if num18 == 0.0 {
            fatalError("Matrix is singular")
        }
        
        let num19 = 1.0 / num18
        var result = inPlace ? matrix : [[Float]](repeating: [Float](repeating: 0.0, count: 2), count: 2)
        
        result[0][0] = num19 * num17
        result[0][1] = -num19 * num15
        result[1][0] = -num19 * num16
        result[1][1] = num19 * num14
        
        return result
    }
}
