//
//  AvatarManager.swift
//  MetaScripts
//
//  Created by ZhaoTianyu on 2024/8/8.
//

import Foundation
import simd

class AvatarManager {
    //    var avatarBonePoint = [AvatarBonePoint](repeating: AvatarBonePoint(), count: 32)
    var avatarBonePoint = [AvatarBonePoint]()
    var poorLowerBodyMode = true
    
    var tall: Float = 336.0
    var tallHead: Float = 0.0
    var tallHeadNeck: Float = 0.0
    var tallNeckSpine: Float = 0.0
    var tallShin: Float = 0.0
    var tallThigh: Float = 0.0
    var tallSpineCrotch: Float = 0.0
    
    var prevTall: Float = 336.0
    var prevTallHead: Float = 0.0
    var prevTallHeadNeck: Float = 0.0
    var prevTallNeckSpine: Float = 0.0
    var prevTallShin: Float = 0.0
    var prevTallSpineCrotch: Float = 0.0
    var prevTallThigh: Float = 0.0
    
    var centerTall: Float = 336.0
    var distanceToPerson: Float = 3.0
    var estimatedScore: Float = 0.0
    var estimatedThreshold: Float = 0.3
    var forwardLowerVec: Vector3 = .zero
    var forwardUpperVec: Vector3 = .zero
    var upperVec: Vector3 = .zero
    var rightUpperVec: Vector3 = .zero
    var downVec: Vector3 = .zero
    var rightLowerVec: Vector3 = .zero
    var lShldrBendF: Vector3 = .forward
    var lForearmBendF: Vector3 = .forward
    var rShldrBendF: Vector3 = .forward
    var rForearmBendF: Vector3 = .forward
    
    var modelData = ModelData()
    
    let footCheckThreshold: Float = 0.5
    let visibleThreshold: Float = 0.05
    let bottomThreshold: Float =  -180.0
    let hypotheticalCamera: Float = 3.0
    let lockFoot = false
    let lockHand = false
    let lockLegs = false
    
    func start() {
        for i in 0..<32 {
            let point = AvatarBonePoint(idx: i)
            avatarBonePoint.append(point)
        }
        
        mapAvatarBone()
    }
    
    private func mapAvatarBone() {
        setInitTPose()
        
        if distanceToPerson == 0 {
            centerTall = 336.0
        } else {
            centerTall = distanceToPerson * 336.0 / hypotheticalCamera
        }
        
        tall = centerTall
        prevTall = centerTall
        
        
        avatarBonePoint[0].transform = modelData.getBoneTransform(.RightUpperArm)
        avatarBonePoint[1].transform = modelData.getBoneTransform(.RightLowerArm)
        avatarBonePoint[2].transform = modelData.getBoneTransform(.RightHand)
        avatarBonePoint[3].transform = modelData.getBoneTransform(.RightThumbIntermediate)
        avatarBonePoint[4].transform = modelData.getBoneTransform(.RightMiddleProximal)
        if (avatarBonePoint[3].transform == nil) || (avatarBonePoint[4].transform == nil) {
            avatarBonePoint[2].enabled = false
            avatarBonePoint[3].enabled = false
            avatarBonePoint[4].enabled = false
        }
        avatarBonePoint[29].transform = modelData.getBoneTransform(.RightShoulder);
        if avatarBonePoint[29].transform == nil {
            avatarBonePoint[29].enabled = false
        }
        avatarBonePoint[5].transform = modelData.getBoneTransform(.LeftUpperArm)
        avatarBonePoint[6].transform = modelData.getBoneTransform(.LeftLowerArm)
        avatarBonePoint[7].transform = modelData.getBoneTransform(.LeftHand)
        avatarBonePoint[8].transform = modelData.getBoneTransform(.LeftThumbIntermediate)
        avatarBonePoint[9].transform = modelData.getBoneTransform(.LeftMiddleProximal)
        if (avatarBonePoint[8].transform == nil) || (avatarBonePoint[9].transform == nil) {
            avatarBonePoint[7].enabled = false
            avatarBonePoint[8].enabled = false
            avatarBonePoint[9].enabled = false
        }
        avatarBonePoint[30].transform = modelData.getBoneTransform(.LeftShoulder)
        if avatarBonePoint[30].transform == nil {
            avatarBonePoint[30].enabled = false
        }
        avatarBonePoint[11].transform = modelData.getBoneTransform(.LeftEye)
        avatarBonePoint[13].transform = modelData.getBoneTransform(.RightEye)
        avatarBonePoint[15].transform = modelData.getBoneTransform(.RightUpperLeg)
        avatarBonePoint[16].transform = modelData.getBoneTransform(.RightLowerLeg)
        avatarBonePoint[17].transform = modelData.getBoneTransform(.RightFoot)
        avatarBonePoint[18].transform = modelData.getBoneTransform(.RightToes)
        if avatarBonePoint[18].transform == nil {
            avatarBonePoint[18].enabled = false
        }
        avatarBonePoint[19].transform = modelData.getBoneTransform(.LeftUpperLeg)
        avatarBonePoint[20].transform = modelData.getBoneTransform(.LeftLowerLeg)
        avatarBonePoint[21].transform = modelData.getBoneTransform(.LeftFoot)
        avatarBonePoint[22].transform = modelData.getBoneTransform(.LeftToes)
        if avatarBonePoint[22].transform == nil {
            avatarBonePoint[22].enabled = false
        }
        avatarBonePoint[23].transform = modelData.getBoneTransform(.Spine)
        avatarBonePoint[24].transform = modelData.getBoneTransform(.Hips)
        avatarBonePoint[25].transform = modelData.getBoneTransform(.Head)
        avatarBonePoint[26].transform = modelData.getBoneTransform(.Neck)
        avatarBonePoint[27].transform = modelData.getBoneTransform(.Spine)
        avatarBonePoint[31].transform = modelData.getBoneTransform(.Chest)
        if avatarBonePoint[31].transform == nil {
            avatarBonePoint[31].enabled = false
        }
//        avatarBonePoint[32].transform = modelData.getBoneTransform(.UpperChest)
        
        let transform = avatarBonePoint[24].transform!
        let transform2 = avatarBonePoint[15].transform!
        if transform.position.y <= transform2.position.y {
            if abs(transform.position.y - transform2.position.y) < 0.1 {
                transform2.position = Vector3(transform2.position.x, transform.position.y - 0.01, transform2.position.z)
            }
        }
        let transform3 = avatarBonePoint[19].transform!
        if transform.position.y <= transform3.position.y {
            if abs(transform.position.y - transform3.position.y) < 0.1 {
                transform3.position = Vector3(transform3.position.x, transform.position.y - 0.01, transform3.position.z)
            }
        }
        
        avatarBonePoint[0].child = avatarBonePoint[1]
        avatarBonePoint[1].child = avatarBonePoint[2]
        if avatarBonePoint[29].enabled {
            avatarBonePoint[29].child = avatarBonePoint[0]
        }
        
        avatarBonePoint[5].child = avatarBonePoint[6]
        avatarBonePoint[6].child = avatarBonePoint[7]
        if (avatarBonePoint[30].enabled) {
            avatarBonePoint[30].child = avatarBonePoint[5]
        }
        avatarBonePoint[15].child = avatarBonePoint[16]
        avatarBonePoint[16].child = avatarBonePoint[17]
        avatarBonePoint[17].child = avatarBonePoint[18]
        avatarBonePoint[17].parent = avatarBonePoint[16]
        avatarBonePoint[19].child = avatarBonePoint[20]
        avatarBonePoint[20].child = avatarBonePoint[21]
        avatarBonePoint[21].child = avatarBonePoint[22]
        avatarBonePoint[21].parent = avatarBonePoint[20]
        if (avatarBonePoint[31].enabled) {
            avatarBonePoint[27].child = avatarBonePoint[31]
            // 胸部
            avatarBonePoint[31].child = avatarBonePoint[26]
            avatarBonePoint[31].child1 = avatarBonePoint[29]
            avatarBonePoint[31].child2 = avatarBonePoint[30]
        } else {
            avatarBonePoint[27].child = avatarBonePoint[26]
        }
        avatarBonePoint[26].child = avatarBonePoint[25]
        
        // 屁股
        avatarBonePoint[24].child = avatarBonePoint[27]
        avatarBonePoint[24].child1 = avatarBonePoint[19]
        avatarBonePoint[24].child2 = avatarBonePoint[15]
        
        // 胸部
//        avatarBonePoint[32].child = avatarBonePoint[26]
//        avatarBonePoint[32].child1 = avatarBonePoint[29]
//        avatarBonePoint[32].child2 = avatarBonePoint[30]
        
        
        let bonePoint24Transform = avatarBonePoint[24].transform!
        let bonePoint19Transform = avatarBonePoint[19].transform!
        let bonePoint15Transform = avatarBonePoint[15].transform!
        let bonePoint26Transform = avatarBonePoint[26].transform!
        let bonePoint23Transform = avatarBonePoint[23].transform!
        
        let forward = triangleNormal(bonePoint24Transform.position, bonePoint19Transform.position, bonePoint15Transform.position)
        self.vector(bonePoint26Transform.position, bonePoint23Transform.position)
        avatarBonePoint.forEach { point5 in
            if let point5Transform = point5.transform {
                point5.initRotation = point5Transform.rotation
                point5.initLocalRotation = point5Transform.localRotation
                
                if
                    let point5Parent = point5.parent,
                    let point5ParentTransform = point5Parent.transform,
                    let point5Child = point5.child,
                    let _ = point5Child.transform {
                    
                    let vector3 = point5ParentTransform.position - point5Transform.position
                    point5.inverse = getInverse(point5, point5Child, forward: vector3)
                    point5.inverseRotation = point5.inverse * point5.initRotation
                    
                } else if let point5Child = point5.child, let _ = point5Child.transform {
                    
                    point5.inverse = getInverse(point5, point5Child, forward: forward)
                    point5.inverseRotation = point5.inverse * point5.initRotation
                }
            }
        }
        
        let point = avatarBonePoint[24]
        let q = Quaternion.lookRotation(forward: forward, up: .up)
        point.inverse = q.inverse
        point.inverseRotation = point.inverse * point.initRotation
        
        let point2 = avatarBonePoint[25]
        let vector2 = Vector3(0, 0, 0.05)
        let point2Transform = point2.transform!
        point2.initRotation = point2Transform.rotation
        point2.inverse = Quaternion.lookRotation(forward: vector2, up: .up).inverse
        point2.inverseRotation = point2.inverse * point2.initRotation
        
        let point3 = avatarBonePoint[7]
        let point3Transform = point3.transform!
        let point9Transform = avatarBonePoint[9].transform!
        let point8Transform = avatarBonePoint[8].transform!
        
        if point3.enabled {
            var vector4 = triangleNormal(point3Transform.position, point9Transform.position, point8Transform.position)
            point3.initRotation = point3Transform.rotation
            point3.inverse = Quaternion.lookRotation(forward: point3Transform.position - point9Transform.position, up: vector4).inverse
            point3.inverseRotation = point3.inverse * point3.initRotation
            
            let point6 = avatarBonePoint[6]
            let point7 = avatarBonePoint[5]
            let point6Transform = point6.transform!
            let point7Transform = point7.transform!
            point6.initRotation = point6Transform.rotation
            point6.inverse = Quaternion.lookRotation(forward: point6Transform.position - point3Transform.position, up: vector4).inverse
            point6.inverseRotation = point6.inverse * point6.initRotation
            vector4 = triangleNormal(point7Transform.position, point6Transform.position, bonePoint23Transform.position)
            point7.initRotation = point7Transform.rotation
            point7.inverse = Quaternion.lookRotation(forward: point7Transform.position - point3Transform.position, up: vector4).inverse
            point7.inverseRotation = point7.inverse * point7.initRotation
        }
        
        let point4 = avatarBonePoint[2]
        if (point4.enabled) {
            let point4Transform = point4.transform!
            let bonePoint4Transform = avatarBonePoint[4].transform!
            let bonePoint3Transform = avatarBonePoint[3].transform!
            
            var vector5 = triangleNormal(point4Transform.position, bonePoint4Transform.position, bonePoint3Transform.position)
            point4.initRotation = point4Transform.rotation
            point4.inverse = Quaternion.lookRotation(forward: point4Transform.position - bonePoint4Transform.position, up: vector5).inverse
            point4.inverseRotation = point4.inverse * point4.initRotation
            
            let point8 = avatarBonePoint[1]
            let point9 = avatarBonePoint[0]
            let point8Transform = point8.transform!
            let point9Transform = point9.transform!
            point8.initRotation = point8Transform.rotation
            
            point8.inverse = Quaternion.lookRotation(forward: point8Transform.position - bonePoint4Transform.position, up: vector5).inverse
            point8.inverseRotation = point8.inverse * point8.initRotation;
            vector5 = triangleNormal(point9Transform.position, point8Transform.position, bonePoint23Transform.position)
            point9.initRotation = point9Transform.rotation
            point9.inverse = Quaternion.lookRotation(forward: point9Transform.position - point4Transform.position, up: vector5).inverse
            point9.inverseRotation = point9.inverse * point9.initRotation
        }
    }
    
    func poseUpdate(keyPoints: [KeyPoint]) {
        if keyPoints.isEmpty {
            fatalError("KeyPoint 数据为空，请检查代码逻辑")
        }
        
        if keyPoints.count < 24 {
            fatalError("KeyPoint数据少于24个，请检查代码逻辑")
        }
        
        for i in 0..<24 {
            let pos3D = keyPoints[i].pos3D
            let score3D = keyPoints[i].score3D
            avatarBonePoint[i].pos3D = pos3D
            avatarBonePoint[i].score3D = score3D
        }
        
        calculateAvatarBones(keyPoints: keyPoints)
        for j in 0..<24 {
            avatarBonePoint[j].visibled = avatarBonePoint[j].score3D >= visibleThreshold
        }
        avatarBonePoint[24].visibled = avatarBonePoint[23].visibled
        avatarBonePoint[25].visibled = avatarBonePoint[11].visibled && avatarBonePoint[13].visibled
        avatarBonePoint[26].visibled = avatarBonePoint[5].visibled && avatarBonePoint[0].visibled
        avatarBonePoint[27].visibled = avatarBonePoint[23].visibled
        avatarBonePoint[28].visibled = avatarBonePoint[23].visibled
        avatarBonePoint[29].visibled = avatarBonePoint[0].visibled
        avatarBonePoint[30].visibled = avatarBonePoint[5].visibled
        avatarBonePoint[31].visibled = avatarBonePoint[23].visibled
        
        if avatarBonePoint[10].visibled && avatarBonePoint[12].visibled {
            tallHead = avatarBonePoint[10].pos3D.distance(to: avatarBonePoint[12].pos3D)
        } else {
            tallHead = prevTallHead
        }
        if avatarBonePoint[25].visibled && avatarBonePoint[26].visibled {
            tallHeadNeck = avatarBonePoint[25].pos3D.distance(to: avatarBonePoint[26].pos3D)
        } else {
            tallHeadNeck = prevTallHeadNeck
        }
        if avatarBonePoint[26].visibled && avatarBonePoint[27].visibled {
            tallNeckSpine = avatarBonePoint[26].pos3D.distance(to: avatarBonePoint[27].pos3D)
        } else {
            tallNeckSpine = prevTallNeckSpine
        }
        var _prevTallThigh: Float = 0.0
        var num2: Float = 0.0
        if avatarBonePoint[20].visibled && avatarBonePoint[21].visibled && !avatarBonePoint[20].lock && !avatarBonePoint[21].lock {
            _prevTallThigh = avatarBonePoint[20].pos3D.distance(to: avatarBonePoint[21].pos3D)
        } else {
            _prevTallThigh = prevTallThigh
        }
        if avatarBonePoint[16].visibled && avatarBonePoint[17].visibled && !avatarBonePoint[16].lock && !avatarBonePoint[17].lock {
            num2 = avatarBonePoint[16].pos3D.distance(to: avatarBonePoint[17].pos3D)
        } else {
            num2 = prevTallThigh
        }
        tallShin = (num2 + _prevTallThigh) / 2.0
        
        var num3: Float = 0.0
        var num4: Float = 0.0
        if (avatarBonePoint[15].visibled && avatarBonePoint[16].visibled) && (!avatarBonePoint[15].lock && !avatarBonePoint[16].lock) {
            num3 = avatarBonePoint[15].pos3D.distance(to: avatarBonePoint[16].pos3D)
        } else {
            num3 = prevTallThigh
        }
        if (avatarBonePoint[19].visibled && avatarBonePoint[20].visibled) && (!avatarBonePoint[19].lock && !avatarBonePoint[20].lock) {
            num4 = avatarBonePoint[19].pos3D.distance(to: avatarBonePoint[20].pos3D)
        } else {
            num4 = prevTallThigh
        }
        tallThigh = (num3 + num4) / 2.0
        
        if ((avatarBonePoint[15].visibled && avatarBonePoint[19].visibled) && (avatarBonePoint[27].visibled && !avatarBonePoint[15].lock)) && !avatarBonePoint[19].lock {
            let vector2 = avatarBonePoint[15].pos3D + avatarBonePoint[19].pos3D / 2
            tallSpineCrotch = avatarBonePoint[27].pos3D.distance(to: vector2)
        } else {
            tallSpineCrotch = prevTallSpineCrotch
        }
        
        let num5 = ((tallHeadNeck + tallNeckSpine) + tallSpineCrotch) + (tallThigh + tallShin)
        tall = (num5 * 0.5) + (prevTall * 0.5)
        let num6 = ((tall / centerTall) - 1) * distanceToPerson
        prevTall = tall;
        prevTallHead = (tallHead * 0.3) + (prevTallHead * 0.7)
        prevTallHeadNeck = (tallHeadNeck * 0.3) + (prevTallHeadNeck * 0.7)
        prevTallNeckSpine = (prevTallNeckSpine * 0.3) + (tallNeckSpine * 0.7)
        prevTallSpineCrotch = (prevTallSpineCrotch * 0.3) + (tallSpineCrotch * 0.7)
        prevTallThigh = (prevTallThigh * 0.3) + (tallThigh * 0.7)
        prevTallShin = (prevTallShin * 0.3) + (tallShin * 0.7)
        var num7: Float = 0
        var num8: Int = 0
        num7 += avatarBonePoint[11].score3D
        num7 += avatarBonePoint[13].score3D
        num7 += avatarBonePoint[10].score3D
        num7 += avatarBonePoint[12].score3D
        num7 += avatarBonePoint[14].score3D
        num8 += 5
        num7 += avatarBonePoint[5].score3D
        num7 += avatarBonePoint[0].score3D
        num7 += avatarBonePoint[23].score3D
        num8 += 3
        if (!poorLowerBodyMode) {
            num7 += avatarBonePoint[19].score3D
            num8 += 1
            num7 += avatarBonePoint[15].score3D
            num8 += 1
            if (!avatarBonePoint[20].lock) {
                num7 += avatarBonePoint[20].score3D
                num8 += 1
            }
            if (!avatarBonePoint[16].lock) {
                num7 += avatarBonePoint[16].score3D
                num8 += 1
            }
            if (!avatarBonePoint[21].lock) {
                num7 += avatarBonePoint[21].score3D
                num8 += 1
            }
            if (!avatarBonePoint[17].lock) {
                num7 += avatarBonePoint[17].score3D
                num8 += 1
            }
            if (!avatarBonePoint[22].lock) {
                num7 += avatarBonePoint[22].score3D
                num8 += 1
            }
            if (!avatarBonePoint[18].lock) {
                num7 += avatarBonePoint[18].score3D
                num8 += 1
            }
            estimatedScore = num7 / Float(num8)
            if (estimatedScore < estimatedThreshold) {
                return
            }
        } else {
            estimatedScore = num7 / Float(num8)
            if (estimatedScore < estimatedThreshold) {
                return
            }
        }
        
        let point = avatarBonePoint[24]
        forwardUpperVec = triangleNormal(avatarBonePoint[23].pos3D, avatarBonePoint[0].pos3D, avatarBonePoint[5].pos3D)
        forwardLowerVec = triangleNormal(avatarBonePoint[23].pos3D, avatarBonePoint[19].pos3D, avatarBonePoint[15].pos3D)
        upperVec = vector(26, 23)
        rightUpperVec = cross(upperVec, forwardUpperVec)
        downVec = vector(28, 23)
        rightLowerVec = cross(forwardLowerVec, downVec)
        
        // TODO 用到了 swift 中不存在的 baseObject
        /*
         var kp = Vector3(0, 0, num6 * zMovementSensitivity)
         kp = (kalmanFilter?.correctAndPredict(kp: kp))!
         this.baseObject.transform.position = this.initPosition + new Vector3(point.Pos3D.x * this.movementSenstivity.x, point.Pos3D.y * this.movementSenstivity.y, (point.Pos3D.z * this.movementSenstivity.z) + kp.z)
         */
        
        point.transform!.rotation = Quaternion.lookRotation(forward: forwardLowerVec, up: -downVec) * point.inverseRotation
//        point.transform!.rotation = Quaternion.lookRotation(forward: Vector3(0.99, -0.03, 0.14), up: -Vector3(-0.10, -1.66, 0.28)) * Quaternion(ix: 0.01918, iy: 0.00000, iz: 0.00000, r: 0.99982)
        
        if ((Vector3.Angle(rightUpperVec, rightLowerVec) < 100) && (Vector3.Angle(upperVec, downVec) > 10)) {
            if (avatarBonePoint[31].enabled) {
                lookAt(27, 31, forwardUpperVec)
                let upwords = triangleNormal(avatarBonePoint[31].pos3D, avatarBonePoint[0].pos3D, avatarBonePoint[5].pos3D)
                lookAt(31, 26, upwords)
            } else {
                lookAt(27, 26, forwardUpperVec);
            }
            let vector3 = triangleNormal(avatarBonePoint[14].pos3D, avatarBonePoint[12].pos3D, avatarBonePoint[10].pos3D)
            let vector1 = avatarBonePoint[26].pos3D - avatarBonePoint[23].pos3D
            if (Vector3.Angle(vector3, upperVec) < 45) {
                lookAt(26, 25, forwardUpperVec)
                // 0.47180, -0.48749, -0.62625, 0.38416
                avatarBonePoint[26].transform?.rotation = Quaternion(ix: 0.47180, iy: -0.48749, iz: -0.62625, r: 0.38416)
                
                let point4 = avatarBonePoint[25]
                let vector5 = avatarBonePoint[14].pos3D - point4.pos3D
                if (Vector3.Angle(vector5, forwardUpperVec) < 60) {
                    print("avatarBonePoint[14].pos3D:\(avatarBonePoint[14].pos3D) - avatarBonePoint[25].pos3D\(point4.pos3D)")
                    print("vector5:\(vector5)")
                    
                    print("avatarBonePoint[14].pos3D:\(avatarBonePoint[14].pos3D), avatarBonePoint[12].pos3D:\(avatarBonePoint[12].pos3D), avatarBonePoint[10].pos3D:\(avatarBonePoint[10].pos3D)")
                    print("vector3:\(vector3)")
                    
                    print("point4.inverseRotation:\(point4.inverseRotation)")
                    point4.transform!.rotation = Quaternion.lookRotation(forward: vector5, up: vector3) * point4.inverseRotation
                    print("point4.transform!.rotation:\(point4.transform!.rotation)")
                }
            }
            let point2 = avatarBonePoint[7];
            if (point2.enabled) {
                if (avatarBonePoint[30].enabled) {
                    lookAt(30, 5, forwardUpperVec)
                }
                let vector6 = triangleNormal(point2.pos3D, avatarBonePoint[9].pos3D, avatarBonePoint[8].pos3D)
                if (getVectorAngle(5, 6, 23) > 5) {
                    lShldrBendF = triangleNormal(5, 6, 23)
                }
                lookAt(5, 6, lShldrBendF)
                let num11 = getVectorAngle(6, 7, 5)
                if (num11 > 5) {
                    lForearmBendF = triangleNormal(6, 7, 5)
                }
                if (num11 < 20) {
                    lookAt(6, 7, vector6)
                } else if (num11 < 90) {
                    let num12: Float = (num11 - 20) / 70
                    let vector7 = (Vector3)((vector6 * (1 - num12)) + (lForearmBendF * num12))
                    lookAt(6, 7, vector7)
                } else {
                    lookAt(6, 7, lForearmBendF)
                }
                lookAt(7, 9, vector6)
            } else {
                lookAt(5, 6, forwardUpperVec)
                lookAt(6, 7, forwardUpperVec)
            }
            let point3 = avatarBonePoint[2]
            if (point3.enabled) {
                if (avatarBonePoint[29].enabled) {
                    lookAt(29, 0, forwardUpperVec)
                }
                let vector8 = triangleNormal(point3.pos3D, avatarBonePoint[4].pos3D, avatarBonePoint[3].pos3D)
                if (getVectorAngle(0, 1, 23) > 5) {
                    rShldrBendF = triangleNormal(0, 1, 23)
                }
                lookAt(0, 1, rShldrBendF)
                let num13 = getVectorAngle(1, 2, 0)
                if (num13 > 5) {
                    rForearmBendF = triangleNormal(1, 2, 0)
                }
                if (num13 < 20) {
                    lookAt(1, 2, vector8)
                } else if (num13 < 90) {
                    let num14 = (num13 - 20) / 70
                    let vector9 = (Vector3)((vector8 * (1 - num14)) + (rForearmBendF * num14))
                    lookAt(1, 2, vector9)
                } else {
                    lookAt(1, 2, rForearmBendF)
                }
                lookAt(2, 4, vector8)
            } else {
                lookAt(0, 1, forwardUpperVec)
                lookAt(1, 2, forwardUpperVec)
            }
        }
        if (!poorLowerBodyMode) {
            legRotate(thighBend: 19, shin: 20, foot: 21, toe: 22)
            legRotate(thighBend: 15, shin: 16, foot: 17, toe: 18)
        } else {
            setDefaultRotation(19, 20, forwardLowerVec)
            setDefaultRotation(20, 21, forwardLowerVec)
            setDefaultRotation(21, 22, forwardLowerVec)
            setDefaultRotation(15, 16, forwardLowerVec)
            setDefaultRotation(16, 17, forwardLowerVec)
            setDefaultRotation(17, 18, forwardLowerVec)
        }
        
        
        avatarBonePoint.forEach { point in
            if let rotation = point.transform?.rotation {
                print("\(rotation.imag.x),\(rotation.imag.y),\(rotation.imag.z),\(rotation.real),")
            }
        }
        
        print("result:")
        print(avatarBonePoint)
    }
}

extension AvatarManager {
    private func setInitTPose() {
        // this.baseObject.transform.position = this.initPosition;
        initTPose(index: 24)
        initTPose(index: 27)
        if (avatarBonePoint[31].enabled) {
            initTPose(index: 31)
        }
        initTPose(index: 26)
        initTPose(index: 25)
        if (avatarBonePoint[30].enabled) {
            initTPose(index: 30)
        }
        if (avatarBonePoint[29].enabled) {
            initTPose(index: 29)
        }
        initTPose(index: 5)
        initTPose(index: 6)
        initTPose(index: 7)
        initTPose(index: 0)
        initTPose(index: 1)
        initTPose(index: 2)
        initTPose(index: 19)
        initTPose(index: 20)
        initTPose(index: 21)
        initTPose(index: 15)
        initTPose(index: 16)
        initTPose(index: 17)
    }
    
    private func initTPose(index: Int) {
        guard let transform = avatarBonePoint[index].transform else {
            return
        }
        
        transform.rotation = avatarBonePoint[index].initRotation
    }
    
    private func checkLeg(piThighBend: Int, piShin: Int, piFoot: Int, piShldrBend: Int, forward: Vector3) {
        let point = avatarBonePoint[piThighBend]
        let point2 = avatarBonePoint[piShin]
        let point3 = avatarBonePoint[piFoot]
        let child = point3.child!
        let point5 = avatarBonePoint[piShldrBend]
        
        if point2.score3D < footCheckThreshold && point.score3D > footCheckThreshold {
            let vector = point.pos3D - point5.pos3D
            if (point.pos3D + vector).y < bottomThreshold {
                child.lock = true
                point3.lock = true
                point2.lock = true
                point.lock = true
            } else {
                child.lock = lockFoot || lockLegs
                point3.lock = lockFoot || lockLegs
                point2.lock = lockLegs
                point.lock = lockLegs
            }
        } else {
            child.lock = lockFoot || lockLegs
            point3.lock = lockFoot || lockLegs
            point2.lock = lockLegs
            point.lock = lockLegs
            
            if point3.score3D < footCheckThreshold && point2.score3D > footCheckThreshold {
                let vector2 = point2.pos3D - point.pos3D
                if (point2.pos3D + vector2).y < bottomThreshold {
                    child.lock = true
                    point3.lock = true
                    point2.lock = true
                } else {
                    child.lock = lockFoot || lockLegs
                    point3.lock = lockFoot || lockLegs
                    point2.lock = lockLegs
                }
            }
            
            if child.score3D < footCheckThreshold && point3.score3D > footCheckThreshold {
                let vector3 = point2.pos3D - point.pos3D
                if (point2.pos3D + vector3).y < bottomThreshold {
                    child.lock = true
                    point3.lock = true
                } else {
                    child.lock = lockFoot || lockLegs
                    point3.lock = lockFoot || lockLegs
                }
            }
        }
    }
    
    private func calculateAvatarBones(keyPoints: [KeyPoint]) {
        var vector13: Vector3
        avatarBonePoint[28].pos3D = (avatarBonePoint[15].pos3D + avatarBonePoint[19].pos3D) / 2.0
        avatarBonePoint[24].pos3D = (avatarBonePoint[23].pos3D + avatarBonePoint[28].pos3D) / 2.0
        let forward = triangleNormal(avatarBonePoint[24].pos3D, avatarBonePoint[19].pos3D, avatarBonePoint[15].pos3D)
        let vector2 = triangleNormal(avatarBonePoint[23].pos3D, avatarBonePoint[0].pos3D, avatarBonePoint[5].pos3D)
        let vector3 = avatarBonePoint[23].pos3D - avatarBonePoint[28].pos3D
        var vector4 = ((avatarBonePoint[0].pos3D + avatarBonePoint[5].pos3D) / 2.0) - avatarBonePoint[23].pos3D
        if avatarBonePoint[5].pos3D.y > -56.0 && avatarBonePoint[0].pos3D.y > -56.0 && avatarBonePoint[19].pos3D.y < -112.0 && avatarBonePoint[15].pos3D.y < -112.0 {
            poorLowerBodyMode = true
        } else {
            poorLowerBodyMode = false
        }
        checkLeg(piThighBend: 19, piShin: 20, piFoot: 21, piShldrBend: 5, forward: forward)
        checkLeg(piThighBend: 15, piShin: 16, piFoot: 17, piShldrBend: 0, forward: forward)
        if avatarBonePoint[30].enabled && avatarBonePoint[29].enabled {
            let vector6 = (avatarBonePoint[12].pos3D + avatarBonePoint[10].pos3D) / 2.0
            let vector7 = vector6 - avatarBonePoint[23].pos3D
            let single1 = (avatarBonePoint[0].pos3D + avatarBonePoint[5].pos3D) / 2.0
            let vector8 = single1 - avatarBonePoint[23].pos3D
            let vector9 = normalize(vector7)
            let vector10 = avatarBonePoint[23].pos3D + (vector9 * simd_dot(vector9, vector8))
            let vector11 = (single1 - vector10) / 2.0
            avatarBonePoint[26].pos3D = vector10 + vector11
            avatarBonePoint[25].pos3D = vector6
            vector13 = avatarBonePoint[0].pos3D - avatarBonePoint[5].pos3D
            let num = simd_length(vector13) / 4.0
            let vector12 = normalize(cross(vector4, vector2))
            avatarBonePoint[29].pos3D = avatarBonePoint[26].pos3D + (vector12 * num)
            avatarBonePoint[30].pos3D = avatarBonePoint[26].pos3D - (vector12 * num)
        } else {
            avatarBonePoint[26].pos3D = (avatarBonePoint[0].pos3D + avatarBonePoint[5].pos3D) / 2.0
            avatarBonePoint[25].pos3D = (avatarBonePoint[12].pos3D + avatarBonePoint[10].pos3D) / 2.0
            avatarBonePoint[29].pos3D = (avatarBonePoint[26].pos3D + avatarBonePoint[0].pos3D) / 2.0
            avatarBonePoint[30].pos3D = (avatarBonePoint[26].pos3D + avatarBonePoint[5].pos3D) / 2.0
        }
        avatarBonePoint[27].pos3D = avatarBonePoint[23].pos3D
        vector4 = avatarBonePoint[26].pos3D - avatarBonePoint[23].pos3D
        vector13 = normalize(vector3) + (normalize(vector4) * 2.0)
        vector13 = vector4 / 2.0
        let introduced15 = normalize(vector13)
        let vector5 = introduced15 * simd_length(vector13)
        avatarBonePoint[31].pos3D = avatarBonePoint[23].pos3D + vector5
    }
    
    private func lookAt(_ index: Int, _ childIndex: Int, _ upwords: Vector3) {
        let point = avatarBonePoint[index]
        guard let transform = point.transform else {
            return
        }
        
        let point2 = avatarBonePoint[childIndex]
        point.transform?.rotation = Quaternion.lookRotation(forward: point.pos3D - point2.pos3D, up: upwords) * point.inverseRotation
        // Time.deltaTime 屏幕刷新每帧时间间隔
        let deltaTime: Float = 0.0
        _ = Quaternion.slerp(from: Quaternion.lookRotation(forward: point.pos3D - point2.pos3D, up: upwords) * point.inverseRotation, to: transform.rotation, t: deltaTime)
    }
    
    private func legRotate(thighBend: Int, shin: Int, foot: Int, toe: Int) {
        if !avatarBonePoint[thighBend].lock {
            let vector = avatarBonePoint[thighBend].pos3D
            let vector2 = avatarBonePoint[shin].pos3D
            let num = -(rightLowerVec.x * vector.x + rightLowerVec.y * vector.y + rightLowerVec.z * vector.z)
            let num2 = -(rightLowerVec.x * vector2.x + rightLowerVec.y * vector2.y + rightLowerVec.z * vector2.z + num) / (rightLowerVec.x * rightLowerVec.x + rightLowerVec.y * rightLowerVec.y + rightLowerVec.z * rightLowerVec.z)
            let upwords = cross(vector2 + num2 * rightLowerVec - vector, rightLowerVec)
            lookAt(thighBend, shin, upwords)
            if !avatarBonePoint[shin].lock {
                let num3 = getVectorAngle(shin, foot, thighBend)
                if num3 < 20 {
                    lookAt(shin, foot, forwardLowerVec)
                } else if num3 >= 20 && num3 < 40 {
                    let num4 = (num3 - 20) / 20
                    let vector4 = forwardLowerVec * (1 - num4) + (self.vector(shin, foot) + self.vector(shin, thighBend)) * num4
                    lookAt(shin, foot, vector4)
                } else {
                    lookAt(shin, foot, self.vector(shin, foot) + self.vector(shin, thighBend))
                }
                if !avatarBonePoint[foot].lock {
                    lookAt(foot, toe, self.vector(shin, foot))
                } else {
                    setDefaultRotation(foot, toe, forwardLowerVec)
                }
            } else {
                setDefaultRotation(shin, foot, forwardLowerVec)
                setDefaultRotation(foot, toe, forwardLowerVec)
            }
        } else {
            setDefaultRotation(thighBend, shin, forwardLowerVec)
            setDefaultRotation(shin, foot, forwardLowerVec)
            setDefaultRotation(foot, toe, forwardLowerVec)
        }
    }
    
    private func setDefaultRotation(_ root: Int, _ child: Int, _ forward: Vector3) {
        guard let _ = avatarBonePoint[root].transform else {
            return
        }
        
        avatarBonePoint[root].transform!.localRotation = Quaternion.lerp(
            avatarBonePoint[root].transform!.localRotation,
            avatarBonePoint[root].initLocalRotation,
            0.05
        )
    }
}

extension AvatarManager {
    private func triangleNormal(_ a: Int, _ b: Int, _ c: Int) -> Vector3 {
        return triangleNormal(
            avatarBonePoint[a].pos3D,
            avatarBonePoint[b].pos3D,
            avatarBonePoint[c].pos3D
        )
    }
    
    private func triangleNormal(_ a: Vector3, _ b: Vector3, _ c: Vector3) -> Vector3 {
        let vector = a - c
        let vector2 = cross(a - b, vector)
        return normalize(vector2)
    }
    
    private func vector(_ a: Int, _ b: Int) -> Vector3 {
        return avatarBonePoint[a].pos3D - avatarBonePoint[b].pos3D
    }
    
    private func vector(_ a: Vector3, _ b: Vector3) -> Vector3 {
        return a - b
    }
    
    private func getVectorAngle(_ a: Int, _ b: Int, _ c: Int) -> Float {
        let vector1 = vector(a, b)
        let vector2 = vector(c, a)
        
        let dotProduct = simd_dot(vector1, vector2)
        let lengthsProduct = simd_length(vector1) * simd_length(vector2)
        
        // Clamp the value to avoid NaN errors due to floating point inaccuracies
        let cosineOfAngle = dotProduct / lengthsProduct
        let angle = acos(min(max(cosineOfAngle, -1.0), 1.0))
        
        // Convert to degrees
        let angleInDegrees = angle * 180 / .pi
        
        return angleInDegrees
    }
    
//    private func getInverse(_ p1: AvatarBonePoint, _ p2: AvatarBonePoint, forward: Vector3) -> Quaternion {
//        let positionDifference = p1.transform!.position - p2.transform!.position
//        let direction = simd_normalize(positionDifference)
//        let lookRotation = Quaternion.lookRotation(forward: direction, up: forward)
//        return simd_inverse(lookRotation)
//    }
    
    private func getInverse(_ p1: AvatarBonePoint, _ p2: AvatarBonePoint, forward: Vector3) -> Quaternion {
        let positionDifference = p1.transform!.position - p2.transform!.position
        let lookRotation = Quaternion.lookRotation(forward: positionDifference, up: forward)
        return lookRotation.inverse
    }
    
//    private Quaternion GetInverse(AvatarBonePoint p1, AvatarBonePoint p2, Vector3 forward)
//        {
//            if ((p1.Transform.position - p2.Transform.position) == Vector3.zero)
//            {
//            }
//            return Quaternion.Inverse(Quaternion.LookRotation(p1.Transform.position - p2.Transform.position, forward));
//        }
}
