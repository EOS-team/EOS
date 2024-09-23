//
//  HumanBodyBone.swift
//  KeyPointCenter
//
//  Created by ZhaoTianyu on 2024/8/9.
//

import Foundation

struct HumanBodyBone: Hashable, Codable {
    enum Name: String, CaseIterable, Codable {
        case Hips = "Hips"
        case LeftUpperLeg = "LeftUpperLeg"
        case RightUpperLeg = "RightUpperLeg"
        case LeftLowerLeg = "LeftLowerLeg"
        case RightLowerLeg = "RightLowerLeg"
        case LeftFoot = "LeftFoot"
        case RightFoot = "RightFoot"
        case Spine = "Spine"
        case Chest = "Chest"
        case Neck = "Neck"
        case Head = "Head"
        case LeftShoulder = "LeftShoulder"
        case RightShoulder = "RightShoulder"
        case LeftUpperArm = "LeftUpperArm"
        case RightUpperArm = "RightUpperArm"
        case LeftLowerArm = "LeftLowerArm"
        case RightLowerArm = "RightLowerArm"
        case LeftHand = "LeftHand"
        case RightHand = "RightHand"
        case LeftToes = "LeftToes"
        case RightToes = "RightToes"
        case LeftEye = "LeftEye"
        case RightEye = "RightEye"
        case Jaw = "Jaw"
        case LeftThumbProximal = "LeftThumbProximal"
        case LeftThumbIntermediate = "LeftThumbIntermediate"
        case LeftThumbDistal = "LeftThumbDistal"
        case LeftIndexProximal = "LeftIndexProximal"
        case LeftIndexIntermediate = "LeftIndexIntermediate"
        case LeftIndexDistal = "LeftIndexDistal"
        case LeftMiddleProximal = "LeftMiddleProximal"
        case LeftMiddleIntermediate = "LeftMiddleIntermediate"
        case LeftMiddleDistal = "LeftMiddleDistal"
        case LeftRingProximal = "LeftRingProximal"
        case LeftRingIntermediate = "LeftRingIntermediate"
        case LeftRingDistal = "LeftRingDistal"
        case LeftLittleProximal = "LeftLittleProximal"
        case LeftLittleIntermediate = "LeftLittleIntermediate"
        case LeftLittleDistal = "LeftLittleDistal"
        case RightThumbProximal = "RightThumbProximal"
        case RightThumbIntermediate = "RightThumbIntermediate"
        case RightThumbDistal = "RightThumbDistal"
        case RightIndexProximal = "RightIndexProximal"
        case RightIndexIntermediate = "RightIndexIntermediate"
        case RightIndexDistal = "RightIndexDistal"
        case RightMiddleProximal = "RightMiddleProximal"
        case RightMiddleIntermediate = "RightMiddleIntermediate"
        case RightMiddleDistal = "RightMiddleDistal"
        case RightRingProximal = "RightRingProximal"
        case RightRingIntermediate = "RightRingIntermediate"
        case RightRingDistal = "RightRingDistal"
        case RightLittleProximal = "RightLittleProximal"
        case RightLittleIntermediate = "RightLittleIntermediate"
        case RightLittleDistal = "RightLittleDistal"
        case UpperChest = "UpperChest"
        case LastBone = "LastBone"
    }
    var name: Name
    
    struct Location: Hashable, Codable {
        var x: Float
        var y: Float
        var z: Float
    }
    var location: Location
    
    struct LocalLocation: Hashable, Codable {
        var x: Float
        var y: Float
        var z: Float
    }
    var localLocation: LocalLocation
    
    struct Rotation: Hashable, Codable {
        var x: Float
        var y: Float
        var z: Float
        var w: Float
    }
    var rotation: Rotation
    
    struct LocalRotation: Hashable, Codable {
        var x: Float
        var y: Float
        var z: Float
        var w: Float
    }
    var localRotation: LocalRotation
}
