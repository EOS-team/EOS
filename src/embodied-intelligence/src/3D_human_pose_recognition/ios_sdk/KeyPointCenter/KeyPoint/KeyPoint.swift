//
//  KeyPoint.swift
//  MetaScripts
//
//  Created by ZhaoTianyu on 2024/7/10.
//

import Foundation
import simd

struct KeyPoint: Hashable {
    var index: Int = 0
    var now3D: Vector3 = Vector3(0, 0, 0)
    var pos3D: Vector3 = Vector3(0, 0, 0)
    var score3D: Float = 0.0
    
    // MARK: Test Function
    static func testData() -> [KeyPoint] {
        guard let filepath = Bundle.main.path(forResource: "input_unity", ofType: "txt") else {
            fatalError("获取 keypoint 测试数据文件路径出错，请检查代码")
        }
        
        return keypointsFromFile(path: filepath)
    }
    
    static private func keypointsFromFile(path: String) -> [KeyPoint] {
        guard let content = try? String(contentsOfFile: path) else {
            fatalError("Error: 无法读取文件。")
        }
        
        let lines = content.split(separator: "\n")
        guard lines.count > 2 else {
            fatalError("Error: 文件格式不符合预期。")
        }
        
        var keypoints: [KeyPoint] = []
        for i in 2..<lines.count {
            let numbers = lines[i].split(separator: ",").map { Float($0.trimmingCharacters(in: .whitespacesAndNewlines))! }
            
            for i in 0..<24 {
                let now3D = simd_float3(numbers[i * 3], numbers[i * 3 + 1], numbers[i * 3 + 2])
                let score3D = numbers[24 * 3 + i]
                keypoints.append(KeyPoint(now3D: now3D, score3D: score3D))
            }
            
//            for j in stride(from: 0, to: numbers.count, by: 4) {
//                let now3D = simd_float3(numbers[j], numbers[j+1], numbers[j+2])
//                let score3D = numbers[j+3]
//                keypoints.append(KeyPoint(now3D: now3D, score3D: score3D))
//            }
        }
        return keypoints
    }
}
