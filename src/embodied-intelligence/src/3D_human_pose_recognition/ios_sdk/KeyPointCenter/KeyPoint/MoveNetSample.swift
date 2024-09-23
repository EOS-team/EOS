//
//  MoveNetSample.swift
//  MetaScripts
//
//  Created by ZhaoTianyu on 2024/7/10.
//

import Foundation

class MoveNetSample {
    private var manager = AvatarManager()
    
    private var clockwise: Int = 0
    private var oneEuroFilter2D = [OneEuroFilter]()
    private var oneEuroFilter3D = [OneEuroFilter]()
    private var OneEuroFilterBeta: Float = 0.005
    private var OneEuroFilterDCutoff: Float = 1.2
    private var OneEuroFilterEnable: Bool = true
    private var OneEuroFilterMinCutoff: Float = 3.5
    private var kalmanFilter2D = [KalmanFilter]()
    private var kalmanFilter3D = [KalmanFilter]()
    private var KalmanFilterEnable: Bool = true
    private var KalmanFilterNoise: Float = 0.4
    private var KalmanFilterTimeInterval: Float = 0.45

    private var lowPassFilter2D = [LowPassFilter]()
    private var lowPassFilter3D = [LowPassFilter]()
    private var LowPassFilterEnable: Bool = true
    private var LowPassFilterNOrder: Int = 7
    private var LowPassFilterSmooth: Float = 0.9
    
    private func initFilters() {
        initKalmanFilter()
        initLowPassFilter()
        initOneEuroFilter()
    }
    
    private func initKalmanFilter() {
        if !kalmanFilter2D.isEmpty {
            kalmanFilter2D.removeAll()
        }
        if !kalmanFilter3D.isEmpty {
            kalmanFilter3D.removeAll()
        }
        for _ in 0..<0x18 {
            let kalmanFilter = KalmanFilter(timeInterval: KalmanFilterTimeInterval, noise: KalmanFilterNoise)
            kalmanFilter2D.append(kalmanFilter)
            kalmanFilter3D.append(kalmanFilter)
        }
    }
    
    private func initLowPassFilter() {
        if !lowPassFilter2D.isEmpty {
            lowPassFilter2D.removeAll()
        }
        if !lowPassFilter3D.isEmpty {
            lowPassFilter3D.removeAll()
        }
        for _ in 0..<0x18 {
            let lowPassFilter = LowPassFilter(order: LowPassFilterNOrder, smooth: LowPassFilterSmooth)
            lowPassFilter2D.append(lowPassFilter)
            lowPassFilter3D.append(lowPassFilter)
        }
    }
    
    private func initOneEuroFilter() {
        if !oneEuroFilter2D.isEmpty {
            oneEuroFilter2D.removeAll()
        }
        if !oneEuroFilter3D.isEmpty {
            oneEuroFilter3D.removeAll()
        }
        for _ in 0..<0x18 {
            let oneEuroFilter = OneEuroFilter(min_cutoff: OneEuroFilterMinCutoff, beta: OneEuroFilterBeta, d_cutoff: OneEuroFilterDCutoff)
            oneEuroFilter2D.append(oneEuroFilter)
            oneEuroFilter3D.append(oneEuroFilter)
        }
    }
    
    func predictPose3D(points: [KeyPoint]) {
        var keyPoints = [KeyPoint]()
        let oefTime: Float = 0.02
        for j in 0..<24 {
            var point = KeyPoint(index: j)
            let input = points[j]
            point.now3D = input.now3D
            point.score3D = input.score3D
            
            if clockwise == 0 {
                point.pos3D = Vector3(-point.now3D.x, -point.now3D.y, point.now3D.z) //不旋转
            } else if clockwise == -1 {
                point.pos3D = Vector3(-point.now3D.y, point.now3D.x, point.now3D.z) //逆时针旋转
            } else if clockwise == 1 {
                point.pos3D = Vector3(point.now3D.y, -point.now3D.x, point.now3D.z) //顺时针旋转
            }
            
            if KalmanFilterEnable {
                point.pos3D = kalmanFilter3D[j].correctAndPredict(kp: point.pos3D)
            }
            if LowPassFilterEnable {
                point.pos3D = lowPassFilter3D[j].correctAndPredict(kp: point.pos3D)
            }
            if OneEuroFilterEnable {
                point.pos3D = oneEuroFilter3D[j].correctAndPredict(kp: point.pos3D, t: oefTime)
            }
            keyPoints.append(point)
        }
        
        manager.poseUpdate(keyPoints: keyPoints)
    }
    
    func start() {
        manager.start()
        
        initFilters()
    }
}

