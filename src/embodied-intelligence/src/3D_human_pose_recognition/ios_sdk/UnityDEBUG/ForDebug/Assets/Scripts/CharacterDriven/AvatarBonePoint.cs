using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class AvatarBonePoint
{
    // Fields
    public Quaternion calcuRotation;
    public AvatarBonePoint Child;
    public bool Enabled;
    public int Error;
    public Quaternion InitLocalRotation;
    public Quaternion InitRotation;
    public Quaternion Inverse;
    public Quaternion InverseRotation;
    public bool Lock;
    public AvatarBonePoint Parent;
    public Vector3 Pos3D;
    public float Score3D;
    public Transform Transform;
    public bool Visibled;
}