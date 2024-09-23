//
//  LowPassFilter.swift
//  MetaScripts
//
//  Created by ZhaoTianyu on 2024/7/10.
//

import Foundation

// LowPassFilter类
final class LowPassFilter {
    // Fields
    private var effectiveCount = 0
    private let FactorialCount = 10
    private var NOrderLPF: Int
    private var prevPos3D: [Vector3]
    private var Smooth: Float

    // Methods
    init() {
        self.prevPos3D = [Vector3](repeating: Vector3(x: 0, y: 0, z: 0), count: 10)
        self.NOrderLPF = 7
        self.Smooth = 0.9
    }

    init(order: Int, smooth: Float) {
        self.prevPos3D = [Vector3](repeating: Vector3(x: 0, y: 0, z: 0), count: 10)
        self.NOrderLPF = min(order, 10)
        self.Smooth = smooth
    }

    func correctAndPredict(kp: Vector3) -> Vector3 {
        self.prevPos3D[0] = kp
        for i in 1..<self.NOrderLPF {
            self.prevPos3D[i] = (self.prevPos3D[i] * (1 - self.Smooth)) + (self.prevPos3D[i - 1] * self.Smooth)
        }
        self.prevPos3D[0] = (self.prevPos3D[0] * (1 - self.Smooth)) + (self.prevPos3D[self.NOrderLPF - 1] * self.Smooth)
        if self.effectiveCount < 10 {
            self.effectiveCount += 1
            return kp
        }
        return self.prevPos3D[0]
    }
}
