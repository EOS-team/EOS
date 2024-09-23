using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using NatML.Examples;
public sealed class TDPAvatarController : MonoBehaviour
{
    // Fields
    [Header("Check when moving the avatar")]
    public bool ApplyMotion = true;
    private Animator AvatarAnimator;
    private AvatarBonePoint[] avatarBonePoint;
    //[Header("NatML Runner")]
    private MoveNetSample NatMLRunner;
    private GameObject baseObject;
    private float bottomThreshold = -180f;
    private float centerHeadSize = 89.6f;
    private float centerTall = 336f;
    [Header("foot IK")]
    public bool useIK = true;
    public float HeelOffset = 0.1f;
    public LayerMask layerMask;
    private Vector3 rayPositionOffset;
    private float rayRange;
    [Header("z - axis movement setting"), Range(1f, 10f)]
    public float DistanceToPerson = 3f;
    private Vector3 downVec;
    [Header("3D pose estimation score")]
    public float EstimatedScore;
    [Range(0f, 1f)]
    public float EstimatedThreshold = 0.3f;
    private float filterNoise = 0.5f;
    private float filterTimeInterval = 0.5f;
    [Range(0f, 1f)]
    public float FootCheckThreshold = 0.2f;
    private float forwardFlag = 1f;
    private Vector3 forwardLowerVec;
    private Vector3 forwardUpperVec;
    private float headSize = 89.6f;
    private float hypotheticalCamera = 3f;
    private Vector3 initHeadPosition;
    private Vector3 initPosition;
    private const int InputImageSize = 280;
    private KalmanFilter kalmanFilter;
    private Vector3 lForearmBendF = Vector3.forward;
    private bool LockFoot;
    private bool LockHand;
    private bool LockLegs;
    private Vector3 lShldrBendF = Vector3.forward;
    private Vector3 movementSenstivity = new Vector3(0.01f, 0.01f, 0.01f);
    private bool PoorLowerBodyMode = true;
    private float prevHeadSize = 89.6f;
    private float prevTall = 336f;
    private float prevTallHead;
    private float prevTallHeadNeck;
    private float prevTallNeckSpine;
    private float prevTallShin;
    private float prevTallSpineCrotch;
    private float prevTallThigh;
    private Vector3 rForearmBendF = Vector3.forward;
    private Vector3 rightLowerVec;
    private Vector3 rightUpperVec;
    private Vector3 rShldrBendF = Vector3.forward;
    private float tall = 336f;
    private float tallHead;
    private float tallHeadNeck;
    private float tallNeckSpine;
    private float tallShin;
    private float tallSpineCrotch;
    private float tallThigh;
    private Vector3 upperVec;
    private float VisibleThreshold = 0.05f;
    private float waistTilt;
    private float avatarLeg = 0.0f;
    private Vector3 pelvicOffset;
    [Range(0f, 1f)]
    public float ZMovementSensitivity = 0.5f;


    private Vector3 prevLeftFoot, prevRightFoot, prevPelvis;

    // Methods
    private void CalculateAvatarBones(KeyPoint[] keyPoints)
    {
        Vector3 vector13;
        this.avatarBonePoint[28].Pos3D = (Vector3)((this.avatarBonePoint[15].Pos3D + this.avatarBonePoint[19].Pos3D) / 2f);
        this.avatarBonePoint[24].Pos3D = (Vector3)((this.avatarBonePoint[23].Pos3D + this.avatarBonePoint[28].Pos3D) / 2f);
        Vector3 forward = this.TriangleNormal(this.avatarBonePoint[24].Pos3D, this.avatarBonePoint[19].Pos3D, this.avatarBonePoint[15].Pos3D);
        Vector3 vector2 = this.TriangleNormal(this.avatarBonePoint[23].Pos3D, this.avatarBonePoint[0].Pos3D, this.avatarBonePoint[5].Pos3D);
        Vector3 vector3 = this.avatarBonePoint[23].Pos3D - this.avatarBonePoint[28].Pos3D;
        Vector3 vector4 = ((Vector3)((this.avatarBonePoint[0].Pos3D + this.avatarBonePoint[5].Pos3D) / 2f)) - this.avatarBonePoint[23].Pos3D;
        if (((this.avatarBonePoint[5].Pos3D.y > -56f) && (this.avatarBonePoint[0].Pos3D.y > -56f)) && ((this.avatarBonePoint[19].Pos3D.y < -112f) && (this.avatarBonePoint[15].Pos3D.y < -112f)))
        {
            this.PoorLowerBodyMode = true;
        }
        else
        {
            this.PoorLowerBodyMode = false;
        }
        this.CheckLeg(19, 20, 21, 5, forward);
        this.CheckLeg(15, 16, 17, 0, forward);
        if (this.avatarBonePoint[30].Enabled && this.avatarBonePoint[29].Enabled)
        {
            Vector3 vector6 = (Vector3)((this.avatarBonePoint[12].Pos3D + this.avatarBonePoint[10].Pos3D) / 2f);
            Vector3 vector7 = vector6 - this.avatarBonePoint[23].Pos3D;
            Vector3 single1 = (this.avatarBonePoint[0].Pos3D + this.avatarBonePoint[5].Pos3D) / 2f;
            Vector3 vector8 = ((Vector3)single1) - this.avatarBonePoint[23].Pos3D;
            Vector3 vector9 = Vector3.Normalize(vector7);
            Vector3 vector10 = this.avatarBonePoint[23].Pos3D + ((Vector3)(vector9 * Vector3.Dot(vector9, vector8)));
            Vector3 vector11 = (Vector3)((single1 - vector10) / 2f);
            this.avatarBonePoint[26].Pos3D = vector10 + vector11;
            this.avatarBonePoint[25].Pos3D = vector6;
            vector13 = this.avatarBonePoint[0].Pos3D - this.avatarBonePoint[5].Pos3D;
            float num = vector13.magnitude / 4f;
            Vector3 vector12 = Vector3.Cross(vector4, vector2).normalized;
            this.avatarBonePoint[29].Pos3D = this.avatarBonePoint[26].Pos3D + ((Vector3)(vector12 * num));
            this.avatarBonePoint[30].Pos3D = this.avatarBonePoint[26].Pos3D - ((Vector3)(vector12 * num));
        }
        else
        {
            this.avatarBonePoint[26].Pos3D = (Vector3)((this.avatarBonePoint[0].Pos3D + this.avatarBonePoint[5].Pos3D) / 2f);
            this.avatarBonePoint[25].Pos3D = (Vector3)((this.avatarBonePoint[12].Pos3D + this.avatarBonePoint[10].Pos3D) / 2f);
            this.avatarBonePoint[29].Pos3D = (Vector3)((this.avatarBonePoint[26].Pos3D + this.avatarBonePoint[0].Pos3D) / 2f);
            this.avatarBonePoint[30].Pos3D = (Vector3)((this.avatarBonePoint[26].Pos3D + this.avatarBonePoint[5].Pos3D) / 2f);
        }
        this.avatarBonePoint[27].Pos3D = this.avatarBonePoint[23].Pos3D;
        vector4 = this.avatarBonePoint[26].Pos3D - this.avatarBonePoint[23].Pos3D;
        vector13 = vector3.normalized + ((Vector3)(vector4.normalized * 2f));
        vector13 = (Vector3)(vector4 / 2f);
        Vector3 introduced15 = vector13.normalized;
        Vector3 vector5 = (Vector3)(introduced15 * vector13.magnitude);
        this.avatarBonePoint[31].Pos3D = this.avatarBonePoint[23].Pos3D + vector5;
    }

    private void CheckLeg(int piThighBend, int piShin, int piFoot, int piShldrBend, Vector3 forward)
    {
        AvatarBonePoint point = this.avatarBonePoint[piThighBend];
        AvatarBonePoint point2 = this.avatarBonePoint[piShin];
        AvatarBonePoint point3 = this.avatarBonePoint[piFoot];
        AvatarBonePoint child = point3.Child;
        AvatarBonePoint point5 = this.avatarBonePoint[piShldrBend];
        if ((point2.Score3D < this.FootCheckThreshold) && (point.Score3D > this.FootCheckThreshold))
        {
            Vector3 vector = point.Pos3D - point5.Pos3D;
            if ((point.Pos3D + vector).y < this.bottomThreshold)
            {
                child.Lock = true;
                point3.Lock = true;
                point2.Lock = true;
                point.Lock = true;
            }
            else
            {
                child.Lock = this.LockFoot || this.LockLegs;
                point3.Lock = this.LockFoot || this.LockLegs;
                point2.Lock = this.LockLegs;
                point.Lock = this.LockLegs;
            }
        }
        else
        {
            child.Lock = this.LockFoot || this.LockLegs;
            point3.Lock = this.LockFoot || this.LockLegs;
            point2.Lock = this.LockLegs;
            point.Lock = this.LockLegs;
            if ((point3.Score3D < this.FootCheckThreshold) && (point2.Score3D > this.FootCheckThreshold))
            {
                Vector3 vector2 = point2.Pos3D - point.Pos3D;
                if ((point2.Pos3D + vector2).y < this.bottomThreshold)
                {
                    child.Lock = true;
                    point3.Lock = true;
                    point2.Lock = true;
                }
                else
                {
                    child.Lock = this.LockFoot || this.LockLegs;
                    point3.Lock = this.LockFoot || this.LockLegs;
                    point2.Lock = this.LockLegs;
                }
            }
            if (((child != null) && (child.Score3D < this.FootCheckThreshold)) && (point3.Score3D > this.FootCheckThreshold))
            {
                Vector3 vector3 = point2.Pos3D - point.Pos3D;
                if ((point2.Pos3D + vector3).y < this.bottomThreshold)
                {
                    child.Lock = true;
                    point3.Lock = true;
                }
                else
                {
                    child.Lock = this.LockFoot || this.LockLegs;
                    point3.Lock = this.LockFoot || this.LockLegs;
                }
            }
        }
    }

    private Quaternion GetInverse(AvatarBonePoint p1, AvatarBonePoint p2, Vector3 forward)
    {
        if ((p1.Transform.position - p2.Transform.position) == Vector3.zero)
        {
        }
        return Quaternion.Inverse(Quaternion.LookRotation(p1.Transform.position - p2.Transform.position, forward));
    }

    private float GetVectorAngle(int a, int b, int c)
    {
        return Vector3.Angle(this.Vector(a, b), this.Vector(c, a));
    }

    private void initTPose(int index)
    {
        if (this.avatarBonePoint[index].Transform != null)
        {
            this.avatarBonePoint[index].Transform.rotation = this.avatarBonePoint[index].InitRotation;
        }
    }

    private void LegRotate(int thighBend, int shin, int foot, int toe)
    {
        if (!this.avatarBonePoint[thighBend].Lock)
        {
            Vector3 vector = this.avatarBonePoint[thighBend].Pos3D;
            Vector3 vector2 = this.avatarBonePoint[shin].Pos3D;
            float num = -(((this.rightLowerVec.x * vector.x) + (this.rightLowerVec.y * vector.y)) + (this.rightLowerVec.z * vector.z));
            float num2 = -((((this.rightLowerVec.x * vector2.x) + (this.rightLowerVec.y * vector2.y)) + (this.rightLowerVec.z * vector2.z)) + num) / (((this.rightLowerVec.x * this.rightLowerVec.x) + (this.rightLowerVec.y * this.rightLowerVec.y)) + (this.rightLowerVec.z * this.rightLowerVec.z));
            Vector3 upwords = Vector3.Cross((vector2 + ((Vector3)(num2 * this.rightLowerVec))) - vector, this.rightLowerVec);
            this.LookAt(thighBend, shin, upwords);
            if (!this.avatarBonePoint[shin].Lock)
            {
                float num3 = this.GetVectorAngle(shin, foot, thighBend);
                if (num3 < 20f)
                {
                    this.LookAt(shin, foot, this.forwardLowerVec);
                }
                else if ((num3 >= 20f) && (num3 < 40f))
                {
                    float num4 = (num3 - 20f) / 20f;
                    Vector3 vector4 = (Vector3)((this.forwardLowerVec * (1f - num4)) + ((this.Vector(shin, foot) + this.Vector(shin, thighBend)) * num4));
                    this.LookAt(shin, foot, vector4);
                }
                else
                {
                    this.LookAt(shin, foot, this.Vector(shin, foot) + this.Vector(shin, thighBend));
                }
                if (!this.avatarBonePoint[foot].Lock)
                {
                    this.LookAt(foot, toe, this.Vector(shin, foot));
                }
                else
                {
                    this.SetDefaultRotation(foot, toe, this.forwardLowerVec);
                }
            }
            else
            {
                this.SetDefaultRotation(shin, foot, this.forwardLowerVec);
                this.SetDefaultRotation(foot, toe, this.forwardLowerVec);
            }
        }
        else
        {
            this.SetDefaultRotation(thighBend, shin, this.forwardLowerVec);
            this.SetDefaultRotation(shin, foot, this.forwardLowerVec);
            this.SetDefaultRotation(foot, toe, this.forwardLowerVec);
        }
    }

    private void LookAt(int index, int childIndex, Vector3 upwords)
    {
        AvatarBonePoint point = this.avatarBonePoint[index];
        if (point.Transform != null)
        {
            AvatarBonePoint point2 = this.avatarBonePoint[childIndex];
            point.Transform.rotation = Quaternion.LookRotation(point.Pos3D - point2.Pos3D, upwords) * point.InverseRotation;
            Quaternion.Slerp(Quaternion.LookRotation(point.Pos3D - point2.Pos3D, upwords) * point.InverseRotation, point.Transform.rotation, Time.deltaTime);
        }
    }

    public bool MapToAvatarBone(Animator animator)
    {
        this.ErrorMessage = "";
        if ((animator != null) && animator.isHuman)
        {
            if (this.AvatarAnimator != null)
            {
                this.SetInitTPose();
            }
            this.AvatarAnimator = animator;
        }
        else
        {
            this.ErrorMessage = "Animator is not human.";
            return false;
        }
        if (this.AvatarAnimator.gameObject == null)
        {
            this.ErrorMessage = "Animator gameObject is null.";
            return false;
        }
        this.baseObject = this.AvatarAnimator.gameObject;
        if (this.DistanceToPerson == 0f)
        {
            this.centerTall = 336f;
            this.centerHeadSize = 44.8f;
        }
        else
        {
            this.centerTall = (this.DistanceToPerson * 336f) / this.hypotheticalCamera;
            this.centerHeadSize = (this.DistanceToPerson * 44.8f) / this.hypotheticalCamera;
        }
        this.tall = this.centerTall;
        this.prevTall = this.centerTall;
        this.headSize = this.centerHeadSize;
        this.prevHeadSize = this.centerHeadSize;
        this.kalmanFilter = new KalmanFilter(this.filterTimeInterval, this.filterNoise);
        this.avatarBonePoint = new AvatarBonePoint[32];
        this.avatarLeg = animator.GetBoneTransform(HumanBodyBones.LeftLowerLeg).localPosition.magnitude + animator.GetBoneTransform(HumanBodyBones.LeftFoot).localPosition.magnitude;
        for (int i = 0; i < 32; i++)
        {
            this.avatarBonePoint[i] = new AvatarBonePoint();
            this.avatarBonePoint[i].Enabled = true;
            this.avatarBonePoint[i].Lock = false;
            this.avatarBonePoint[i].Error = 0;
        }
        try
        {
            this.avatarBonePoint[0].Transform = animator.GetBoneTransform(HumanBodyBones.RightUpperArm);
            this.avatarBonePoint[1].Transform = animator.GetBoneTransform(HumanBodyBones.RightLowerArm);
            this.avatarBonePoint[2].Transform = animator.GetBoneTransform(HumanBodyBones.RightHand);
            this.avatarBonePoint[3].Transform = animator.GetBoneTransform(HumanBodyBones.RightThumbIntermediate);
            this.avatarBonePoint[4].Transform = animator.GetBoneTransform(HumanBodyBones.RightMiddleProximal);
            if ((this.avatarBonePoint[3].Transform == null) || (this.avatarBonePoint[4].Transform == null))
            {
                this.avatarBonePoint[2].Enabled = false;
                this.avatarBonePoint[3].Enabled = false;
                this.avatarBonePoint[4].Enabled = false;
            }
            this.avatarBonePoint[29].Transform = animator.GetBoneTransform(HumanBodyBones.RightShoulder);
            if (this.avatarBonePoint[29].Transform == null)
            {
                this.avatarBonePoint[29].Enabled = false;
            }
            this.avatarBonePoint[5].Transform = animator.GetBoneTransform(HumanBodyBones.LeftUpperArm);
            this.avatarBonePoint[6].Transform = animator.GetBoneTransform(HumanBodyBones.LeftLowerArm);
            this.avatarBonePoint[7].Transform = animator.GetBoneTransform(HumanBodyBones.LeftHand);
            this.avatarBonePoint[8].Transform = animator.GetBoneTransform(HumanBodyBones.LeftThumbIntermediate);
            this.avatarBonePoint[9].Transform = animator.GetBoneTransform(HumanBodyBones.LeftMiddleProximal);
            if ((this.avatarBonePoint[8].Transform == null) || (this.avatarBonePoint[9].Transform == null))
            {
                this.avatarBonePoint[7].Enabled = false;
                this.avatarBonePoint[8].Enabled = false;
                this.avatarBonePoint[9].Enabled = false;
            }
            this.avatarBonePoint[30].Transform = animator.GetBoneTransform(HumanBodyBones.LeftShoulder);
            if (this.avatarBonePoint[30].Transform == null)
            {
                this.avatarBonePoint[30].Enabled = false;
            }
            this.avatarBonePoint[11].Transform = animator.GetBoneTransform(HumanBodyBones.LeftEye);
            this.avatarBonePoint[13].Transform = animator.GetBoneTransform(HumanBodyBones.RightEye);
            this.avatarBonePoint[15].Transform = animator.GetBoneTransform(HumanBodyBones.RightUpperLeg);
            this.avatarBonePoint[16].Transform = animator.GetBoneTransform(HumanBodyBones.RightLowerLeg);
            this.avatarBonePoint[17].Transform = animator.GetBoneTransform(HumanBodyBones.RightFoot);
            this.avatarBonePoint[18].Transform = animator.GetBoneTransform(HumanBodyBones.RightToes);
            if (this.avatarBonePoint[18].Transform == null)
            {
                this.avatarBonePoint[18].Enabled = false;
            }
            this.avatarBonePoint[19].Transform = animator.GetBoneTransform(HumanBodyBones.LeftUpperLeg);
            this.avatarBonePoint[20].Transform = animator.GetBoneTransform(HumanBodyBones.LeftLowerLeg);
            this.avatarBonePoint[21].Transform = animator.GetBoneTransform(HumanBodyBones.LeftFoot);
            this.avatarBonePoint[22].Transform = animator.GetBoneTransform(HumanBodyBones.LeftToes);
            if (this.avatarBonePoint[22].Transform == null)
            {
                this.avatarBonePoint[22].Enabled = false;
            }
            this.avatarBonePoint[23].Transform = animator.GetBoneTransform(HumanBodyBones.Spine);
            this.avatarBonePoint[24].Transform = animator.GetBoneTransform(HumanBodyBones.Hips);
            this.avatarBonePoint[25].Transform = animator.GetBoneTransform(HumanBodyBones.Head);
            this.avatarBonePoint[26].Transform = animator.GetBoneTransform(HumanBodyBones.Neck);
            this.avatarBonePoint[27].Transform = animator.GetBoneTransform(HumanBodyBones.Spine);
            this.avatarBonePoint[31].Transform = animator.GetBoneTransform(HumanBodyBones.Chest);
            if (this.avatarBonePoint[31].Transform == null)
            {
                this.avatarBonePoint[31].Enabled = false;
            }
            Transform transform = this.avatarBonePoint[24].Transform;
            Transform transform2 = this.avatarBonePoint[15].Transform;
            this.forwardFlag = 1f;
            if (transform.position.y <= transform2.position.y)
            {
                if (Mathf.Abs(transform.position.y - transform2.position.y) < 0.1f)
                {
                    transform2.position = new Vector3(transform2.position.x, transform.position.y - 0.01f, transform2.position.z);
                }
                this.forwardFlag = -1f;
            }
            Transform transform3 = this.avatarBonePoint[19].Transform;
            if (transform.position.y <= transform3.position.y)
            {
                if (Mathf.Abs(transform.position.y - transform3.position.y) < 0.1f)
                {
                    transform3.position = new Vector3(transform3.position.x, transform.position.y - 0.01f, transform3.position.z);
                }
                this.forwardFlag = -1f;
            }
        }
        catch (Exception exception)
        {
            Debug.Log(exception);
            this.ErrorMessage = "Failed to set the bone.\r\n" + exception.Message;
            return false;
        }
        this.avatarBonePoint[0].Child = this.avatarBonePoint[1];
        this.avatarBonePoint[1].Child = this.avatarBonePoint[2];
        if (this.avatarBonePoint[29].Enabled)
        {
            this.avatarBonePoint[29].Child = this.avatarBonePoint[0];
        }
        this.avatarBonePoint[5].Child = this.avatarBonePoint[6];
        this.avatarBonePoint[6].Child = this.avatarBonePoint[7];
        if (this.avatarBonePoint[30].Enabled)
        {
            this.avatarBonePoint[30].Child = this.avatarBonePoint[5];
        }
        this.avatarBonePoint[15].Child = this.avatarBonePoint[16];
        this.avatarBonePoint[16].Child = this.avatarBonePoint[17];
        this.avatarBonePoint[17].Child = this.avatarBonePoint[18];
        this.avatarBonePoint[17].Parent = this.avatarBonePoint[16];
        this.avatarBonePoint[19].Child = this.avatarBonePoint[20];
        this.avatarBonePoint[20].Child = this.avatarBonePoint[21];
        this.avatarBonePoint[21].Child = this.avatarBonePoint[22];
        this.avatarBonePoint[21].Parent = this.avatarBonePoint[20];
        if (this.avatarBonePoint[31].Enabled)
        {
            this.avatarBonePoint[27].Child = this.avatarBonePoint[31];
            this.avatarBonePoint[31].Child = this.avatarBonePoint[26];
        }
        else
        {
            this.avatarBonePoint[27].Child = this.avatarBonePoint[26];
        }
        this.avatarBonePoint[26].Child = this.avatarBonePoint[25];
        try
        {
            int i = 0;
            Vector3 forward = this.TriangleNormal(this.avatarBonePoint[24].Transform.position, this.avatarBonePoint[19].Transform.position, this.avatarBonePoint[15].Transform.position);
            this.Vector(this.avatarBonePoint[26].Transform.position, this.avatarBonePoint[23].Transform.position);
            foreach (AvatarBonePoint point5 in this.avatarBonePoint)
            {
                i++;
                Debug.Log(i);
                if (point5.Transform != null)
                {
                    point5.InitRotation = point5.Transform.rotation;
                    point5.InitLocalRotation = point5.Transform.localRotation;
                    if (((point5.Parent != null) && (point5.Parent.Transform != null)) && (point5.Child.Transform != null))
                    {
                        Vector3 vector3 = point5.Parent.Transform.position - point5.Transform.position;
                        point5.Inverse = this.GetInverse(point5, point5.Child, vector3);
                        point5.InverseRotation = point5.Inverse * point5.InitRotation;
                    }
                    else if ((point5.Child != null) && (point5.Child.Transform != null))
                    {
                        point5.Inverse = this.GetInverse(point5, point5.Child, forward);
                        point5.InverseRotation = point5.Inverse * point5.InitRotation;
                    }
                }
            }

            AvatarBonePoint point = this.avatarBonePoint[24];
            this.initPosition = this.baseObject.transform.position;
            Quaternion q = Quaternion.LookRotation(forward);
            point.Inverse = Quaternion.Inverse(q);
            point.InverseRotation = point.Inverse * point.InitRotation;

            AvatarBonePoint point2 = this.avatarBonePoint[25];
            this.initHeadPosition = point2.Transform.position;
            point2.InitRotation = this.avatarBonePoint[25].Transform.rotation;
            Vector3 vector2 = new Vector3(0f, 0f, 0.05f);
            point2.Inverse = Quaternion.Inverse(Quaternion.LookRotation(vector2));
            point2.InverseRotation = point2.Inverse * point2.InitRotation;
            AvatarBonePoint point3 = this.avatarBonePoint[7];
            if (point3.Enabled)
            {
                Vector3 vector4 = this.TriangleNormal(point3.Transform.position, this.avatarBonePoint[9].Transform.position, this.avatarBonePoint[8].Transform.position);
                point3.InitRotation = point3.Transform.rotation;
                point3.Inverse = Quaternion.Inverse(Quaternion.LookRotation(point3.Transform.position - this.avatarBonePoint[9].Transform.position, vector4));
                point3.InverseRotation = point3.Inverse * point3.InitRotation;
                AvatarBonePoint point6 = this.avatarBonePoint[6];
                AvatarBonePoint point7 = this.avatarBonePoint[5];
                point6.InitRotation = point6.Transform.rotation;
                point6.Inverse = Quaternion.Inverse(Quaternion.LookRotation(point6.Transform.position - point3.Transform.position, vector4));
                point6.InverseRotation = point6.Inverse * point6.InitRotation;
                vector4 = this.TriangleNormal(point7.Transform.position, point6.Transform.position, this.avatarBonePoint[23].Transform.position);
                point7.InitRotation = point7.Transform.rotation;
                point7.Inverse = Quaternion.Inverse(Quaternion.LookRotation(point7.Transform.position - point3.Transform.position, vector4));
                point7.InverseRotation = point7.Inverse * point7.InitRotation;
            }
            AvatarBonePoint point4 = this.avatarBonePoint[2];
            if (point4.Enabled)
            {
                Vector3 vector5 = this.TriangleNormal(point4.Transform.position, this.avatarBonePoint[4].Transform.position, this.avatarBonePoint[3].Transform.position);
                point4.InitRotation = point4.Transform.rotation;
                point4.Inverse = Quaternion.Inverse(Quaternion.LookRotation(point4.Transform.position - this.avatarBonePoint[4].Transform.position, vector5));
                point4.InverseRotation = point4.Inverse * point4.InitRotation;
                AvatarBonePoint point8 = this.avatarBonePoint[1];
                AvatarBonePoint point9 = this.avatarBonePoint[0];
                point8.InitRotation = point8.Transform.rotation;
                point8.Inverse = Quaternion.Inverse(Quaternion.LookRotation(point8.Transform.position - point4.Transform.position, vector5));
                point8.InverseRotation = point8.Inverse * point8.InitRotation;
                vector5 = this.TriangleNormal(point9.Transform.position, point8.Transform.position, this.avatarBonePoint[23].Transform.position);
                point9.InitRotation = point9.Transform.rotation;
                point9.Inverse = Quaternion.Inverse(Quaternion.LookRotation(point9.Transform.position - point4.Transform.position, vector5));
                point9.InverseRotation = point9.Inverse * point9.InitRotation;
            }
        }
        catch (Exception exception2)
        {
            Debug.Log(exception2);
            this.ErrorMessage = "Failed to set the bone.\r\n" + exception2.Message;
            return false;
        }
        return true;
    }

    public void PoseUpdate(KeyPoint[] keyPoints)
    {
        for (int i = 0; i < 24; i++)
        {
            this.avatarBonePoint[i].Pos3D = keyPoints[i].Pos3D;
            this.avatarBonePoint[i].Score3D = keyPoints[i].Score3D;
        }
        this.CalculateAvatarBones(keyPoints);
        for (int j = 0; j < 24; j++)
        {
            this.avatarBonePoint[j].Visibled = this.avatarBonePoint[j].Score3D >= this.VisibleThreshold;
        }
        this.avatarBonePoint[25].Visibled = this.avatarBonePoint[11].Visibled && this.avatarBonePoint[13].Visibled;
        this.avatarBonePoint[26].Visibled = this.avatarBonePoint[5].Visibled && this.avatarBonePoint[0].Visibled;
        this.avatarBonePoint[27].Visibled = this.avatarBonePoint[23].Visibled;
        this.avatarBonePoint[24].Visibled = this.avatarBonePoint[23].Visibled;
        this.avatarBonePoint[31].Visibled = this.avatarBonePoint[23].Visibled;
        this.avatarBonePoint[28].Visibled = this.avatarBonePoint[23].Visibled;
        this.avatarBonePoint[30].Visibled = this.avatarBonePoint[5].Visibled;
        this.avatarBonePoint[29].Visibled = this.avatarBonePoint[0].Visibled;
        if (this.avatarBonePoint[10].Visibled && this.avatarBonePoint[12].Visibled)
        {
            this.tallHead = Vector3.Distance(this.avatarBonePoint[10].Pos3D, this.avatarBonePoint[12].Pos3D);
        }
        else
        {
            this.tallHead = this.prevTallHead;
        }
        if (this.avatarBonePoint[25].Visibled && this.avatarBonePoint[26].Visibled)
        {
            this.tallHeadNeck = Vector3.Distance(this.avatarBonePoint[25].Pos3D, this.avatarBonePoint[26].Pos3D);
        }
        else
        {
            this.tallHeadNeck = this.prevTallHeadNeck;
        }
        if (this.avatarBonePoint[26].Visibled && this.avatarBonePoint[27].Visibled)
        {
            this.tallNeckSpine = Vector3.Distance(this.avatarBonePoint[26].Pos3D, this.avatarBonePoint[27].Pos3D);
        }
        else
        {
            this.tallNeckSpine = this.prevTallNeckSpine;
        }
        float prevTallThigh = 0f;
        float num2 = 0f;
        if ((this.avatarBonePoint[20].Visibled && this.avatarBonePoint[21].Visibled) && (!this.avatarBonePoint[20].Lock && !this.avatarBonePoint[21].Lock))
        {
            prevTallThigh = Vector3.Distance(this.avatarBonePoint[20].Pos3D, this.avatarBonePoint[21].Pos3D);
        }
        else
        {
            prevTallThigh = this.prevTallThigh;
        }
        if ((this.avatarBonePoint[16].Visibled && this.avatarBonePoint[17].Visibled) && (!this.avatarBonePoint[16].Lock && !this.avatarBonePoint[17].Lock))
        {
            num2 = Vector3.Distance(this.avatarBonePoint[16].Pos3D, this.avatarBonePoint[17].Pos3D);
        }
        else
        {
            num2 = this.prevTallThigh;
        }
        this.tallShin = (num2 + prevTallThigh) / 2f;
        float num3 = 0f;
        float num4 = 0f;
        if ((this.avatarBonePoint[15].Visibled && this.avatarBonePoint[16].Visibled) && (!this.avatarBonePoint[15].Lock && !this.avatarBonePoint[16].Lock))
        {
            num3 = Vector3.Distance(this.avatarBonePoint[15].Pos3D, this.avatarBonePoint[16].Pos3D);
        }
        else
        {
            num3 = this.prevTallThigh;
        }
        if ((this.avatarBonePoint[19].Visibled && this.avatarBonePoint[20].Visibled) && (!this.avatarBonePoint[19].Lock && !this.avatarBonePoint[20].Lock))
        {
            num4 = Vector3.Distance(this.avatarBonePoint[19].Pos3D, this.avatarBonePoint[20].Pos3D);
        }
        else
        {
            num4 = this.prevTallThigh;
        }
        this.tallThigh = (num3 + num4) / 2f;
        if (((this.avatarBonePoint[15].Visibled && this.avatarBonePoint[19].Visibled) && (this.avatarBonePoint[27].Visibled && !this.avatarBonePoint[15].Lock)) && !this.avatarBonePoint[19].Lock)
        {
            Vector3 vector2 = (Vector3)((this.avatarBonePoint[15].Pos3D + this.avatarBonePoint[19].Pos3D) / 2f);
            this.tallSpineCrotch = Vector3.Distance(this.avatarBonePoint[27].Pos3D, vector2);
        }
        else
        {
            this.tallSpineCrotch = this.prevTallSpineCrotch;
        }
        float num5 = ((this.tallHeadNeck + this.tallNeckSpine) + this.tallSpineCrotch) + (this.tallThigh + this.tallShin);
        this.tall = (num5 * 0.5f) + (this.prevTall * 0.5f);
        float num6 = ((this.tall / this.centerTall) - 1f) * this.DistanceToPerson;
        this.prevTall = this.tall;
        this.prevTallHead = (this.tallHead * 0.3f) + (this.prevTallHead * 0.7f);
        this.prevTallHeadNeck = (this.tallHeadNeck * 0.3f) + (this.prevTallHeadNeck * 0.7f);
        this.prevTallNeckSpine = (this.prevTallNeckSpine * 0.3f) + (this.tallNeckSpine * 0.7f);
        this.prevTallSpineCrotch = (this.prevTallSpineCrotch * 0.3f) + (this.tallSpineCrotch * 0.7f);
        this.prevTallThigh = (this.prevTallThigh * 0.3f) + (this.tallThigh * 0.7f);
        this.prevTallShin = (this.prevTallShin * 0.3f) + (this.tallShin * 0.7f);
        float num7 = 0f;
        int num8 = 0;
        num7 += this.avatarBonePoint[11].Score3D;
        num7 += this.avatarBonePoint[13].Score3D;
        num7 += this.avatarBonePoint[10].Score3D;
        num7 += this.avatarBonePoint[12].Score3D;
        num7 += this.avatarBonePoint[14].Score3D;
        num8 += 5;
        num7 += this.avatarBonePoint[5].Score3D;
        num7 += this.avatarBonePoint[0].Score3D;
        num7 += this.avatarBonePoint[23].Score3D;
        num8 += 3;
        if (!this.PoorLowerBodyMode)
        {
            num7 += this.avatarBonePoint[19].Score3D;
            num8++;
            num7 += this.avatarBonePoint[15].Score3D;
            num8++;
            if (!this.avatarBonePoint[20].Lock)
            {
                num7 += this.avatarBonePoint[20].Score3D;
                num8++;
            }
            if (!this.avatarBonePoint[16].Lock)
            {
                num7 += this.avatarBonePoint[16].Score3D;
                num8++;
            }
            if (!this.avatarBonePoint[21].Lock)
            {
                num7 += this.avatarBonePoint[21].Score3D;
                num8++;
            }
            if (!this.avatarBonePoint[17].Lock)
            {
                num7 += this.avatarBonePoint[17].Score3D;
                num8++;
            }
            if (!this.avatarBonePoint[22].Lock)
            {
                num7 += this.avatarBonePoint[22].Score3D;
                num8++;
            }
            if (!this.avatarBonePoint[18].Lock)
            {
                num7 += this.avatarBonePoint[18].Score3D;
                num8++;
            }
            this.EstimatedScore = num7 / ((float)num8);
            if (this.EstimatedScore < this.EstimatedThreshold)
            {
                return;
            }
        }
        else
        {
            this.EstimatedScore = num7 / ((float)num8);
            if (this.EstimatedScore < this.EstimatedThreshold)
            {
                return;
            }
        }
        AvatarBonePoint point = this.avatarBonePoint[24];
        this.forwardUpperVec = this.TriangleNormal(this.avatarBonePoint[23].Pos3D, this.avatarBonePoint[0].Pos3D, this.avatarBonePoint[5].Pos3D);
        this.forwardLowerVec = this.TriangleNormal(this.avatarBonePoint[23].Pos3D, this.avatarBonePoint[19].Pos3D, this.avatarBonePoint[15].Pos3D);
        this.upperVec = this.Vector(26, 23);
        this.rightUpperVec = Vector3.Cross(this.upperVec, this.forwardUpperVec);
        this.downVec = this.Vector(28, 23);
        this.rightLowerVec = Vector3.Cross(this.forwardLowerVec, this.downVec);
        Vector3 kp = new Vector3(0f, 0f, num6 * this.ZMovementSensitivity);
        kp = this.kalmanFilter.CorrectAndPredict(kp);
        //Vector3 scaledRoot = point.Pos3D * (this.avatarLeg / (this.prevTallShin + this.prevTallThigh));
        //if (point.Visibled && pelvicOffset.magnitude == 0)
        //    pelvicOffset = this.initPosition - scaledRoot;
        //Debug.Log(pelvicOffset);
        this.baseObject.transform.position = this.initPosition + new Vector3(point.Pos3D.x * this.movementSenstivity.x, point.Pos3D.y * this.movementSenstivity.y, (point.Pos3D.z * this.movementSenstivity.z) + kp.z);//pelvicOffset + scaledRoot;//
        point.Transform.rotation = Quaternion.LookRotation(this.forwardLowerVec, -this.downVec) * point.InverseRotation;

        //Quaternion tmp_rot = point.Transform.rotation * Quaternion.Inverse(point.InitRotation) * this.avatarBonePoint[25].InitRotation;

        if ((Vector3.Angle(this.rightUpperVec, this.rightLowerVec) < 100f) && (Vector3.Angle(this.upperVec, this.downVec) > 10f))
        {
            if (this.avatarBonePoint[31].Enabled)
            {
                this.LookAt(27, 31, this.forwardUpperVec);
                Vector3 upwords = this.TriangleNormal(this.avatarBonePoint[31].Pos3D, this.avatarBonePoint[0].Pos3D, this.avatarBonePoint[5].Pos3D);
                this.LookAt(31, 26, upwords);
            }
            else
            {
                this.LookAt(27, 26, this.forwardUpperVec);
            }
            Vector3 vector3 = this.TriangleNormal(this.avatarBonePoint[14].Pos3D, this.avatarBonePoint[12].Pos3D, this.avatarBonePoint[10].Pos3D);
            Vector3 vector1 = this.avatarBonePoint[26].Pos3D - this.avatarBonePoint[23].Pos3D;
            if (Vector3.Angle(vector3, this.upperVec) < 45f)
            {
                this.LookAt(26, 25, this.forwardUpperVec);

                Quaternion q = Quaternion.Inverse(this.avatarBonePoint[26].InitRotation);
                Quaternion tmp_rot = this.avatarBonePoint[26].Transform.rotation * q * this.avatarBonePoint[25].Transform.rotation;

                AvatarBonePoint point4 = this.avatarBonePoint[25];
                Vector3 vector5 = this.avatarBonePoint[14].Pos3D - point4.Pos3D;
                if (Vector3.Angle(vector5, this.forwardUpperVec) < 60f)
                {
                    point4.Transform.rotation = Quaternion.LookRotation(vector5, vector3) * point4.InverseRotation;
                }
            }
            AvatarBonePoint point2 = this.avatarBonePoint[7];
            if (point2.Enabled)
            {
                if (this.avatarBonePoint[30].Enabled)
                {
                    this.LookAt(30, 5, this.forwardUpperVec);
                }
                Vector3 vector6 = this.TriangleNormal(point2.Pos3D, this.avatarBonePoint[9].Pos3D, this.avatarBonePoint[8].Pos3D);
                if (this.GetVectorAngle(5, 6, 23) > 5f)
                {
                    this.lShldrBendF = this.TriangleNormal(5, 6, 23);
                }
                this.LookAt(5, 6, this.lShldrBendF);
                float num11 = this.GetVectorAngle(6, 7, 5);
                if (num11 > 5f)
                {
                    this.lForearmBendF = this.TriangleNormal(6, 7, 5);
                }
                if (num11 < 20f)
                {
                    this.LookAt(6, 7, vector6);
                }
                else if (num11 < 90f)
                {
                    float num12 = (num11 - 20f) / 70f;
                    Vector3 vector7 = (Vector3)((vector6 * (1f - num12)) + (this.lForearmBendF * num12));
                    this.LookAt(6, 7, vector7);
                }
                else
                {
                    this.LookAt(6, 7, this.lForearmBendF);
                }
                this.LookAt(7, 9, vector6);
            }
            else
            {
                this.LookAt(5, 6, this.forwardUpperVec);
                this.LookAt(6, 7, this.forwardUpperVec);
            }
            AvatarBonePoint point3 = this.avatarBonePoint[2];
            if (point3.Enabled)
            {
                if (this.avatarBonePoint[29].Enabled)
                {
                    this.LookAt(29, 0, this.forwardUpperVec);
                }
                Vector3 vector8 = this.TriangleNormal(point3.Pos3D, this.avatarBonePoint[4].Pos3D, this.avatarBonePoint[3].Pos3D);
                if (this.GetVectorAngle(0, 1, 23) > 5f)
                {
                    this.rShldrBendF = this.TriangleNormal(0, 1, 23);
                }
                this.LookAt(0, 1, this.rShldrBendF);
                float num13 = this.GetVectorAngle(1, 2, 0);
                if (num13 > 5f)
                {
                    this.rForearmBendF = this.TriangleNormal(1, 2, 0);
                }
                if (num13 < 20f)
                {
                    this.LookAt(1, 2, vector8);
                }
                else if (num13 < 90f)
                {
                    float num14 = (num13 - 20f) / 70f;
                    Vector3 vector9 = (Vector3)((vector8 * (1f - num14)) + (this.rForearmBendF * num14));
                    this.LookAt(1, 2, vector9);
                }
                else
                {
                    this.LookAt(1, 2, this.rForearmBendF);
                }
                this.LookAt(2, 4, vector8);
            }
            else
            {
                this.LookAt(0, 1, this.forwardUpperVec);
                this.LookAt(1, 2, this.forwardUpperVec);
            }
        }
        if (!this.PoorLowerBodyMode)
        {
            this.LegRotate(19, 20, 21, 22);
            this.LegRotate(15, 16, 17, 18);
        }
        else
        {
            this.SetDefaultRotation(19, 20, this.forwardLowerVec);
            this.SetDefaultRotation(20, 21, this.forwardLowerVec);
            this.SetDefaultRotation(21, 22, this.forwardLowerVec);
            this.SetDefaultRotation(15, 16, this.forwardLowerVec);
            this.SetDefaultRotation(16, 17, this.forwardLowerVec);
            this.SetDefaultRotation(17, 18, this.forwardLowerVec);
        }
        Debug.Log(this.avatarBonePoint[26].Transform.rotation);
    }

    private void SetDefaultRotation(int root, int child, Vector3 forward)
    {
        if (this.avatarBonePoint[root].Transform != null)
        {
            this.avatarBonePoint[root].Transform.localRotation = Quaternion.Lerp(this.avatarBonePoint[root].Transform.localRotation, this.avatarBonePoint[root].InitLocalRotation, 0.05f);
        }
    }

    public void SetInitTPose()
    {
        this.baseObject.transform.position = this.initPosition;
        this.initTPose(24);
        this.initTPose(27);
        if (this.avatarBonePoint[31].Enabled)
        {
            this.initTPose(31);
        }
        this.initTPose(26);
        this.initTPose(25);
        if (this.avatarBonePoint[30].Enabled)
        {
            this.initTPose(30);
        }
        if (this.avatarBonePoint[29].Enabled)
        {
            this.initTPose(29);
        }
        this.initTPose(5);
        this.initTPose(6);
        this.initTPose(7);
        this.initTPose(0);
        this.initTPose(1);
        this.initTPose(2);
        this.initTPose(19);
        this.initTPose(20);
        this.initTPose(21);
        this.initTPose(15);
        this.initTPose(16);
        this.initTPose(17);
    }

    private void Start()
    {
        AvatarAnimator = this.GetComponent<Animator>();
        float num = 0.005f;
        this.movementSenstivity = new Vector3(num, num, num);
        if (this.AvatarAnimator != null)
        {
            Animator avatarAnimator = this.AvatarAnimator;
            this.AvatarAnimator = null;
            this.MapToAvatarBone(avatarAnimator);
            pelvicOffset = Vector3.zero;
        }
        prevLeftFoot = AvatarAnimator.GetBoneTransform(HumanBodyBones.LeftFoot).position;
        prevRightFoot = AvatarAnimator.GetBoneTransform(HumanBodyBones.RightFoot).position;

        // init ray trace
        rayPositionOffset = new Vector3(0, AvatarAnimator.GetBoneTransform(HumanBodyBones.LeftFoot).localPosition.magnitude / 3, 0);
        rayRange = AvatarAnimator.GetBoneTransform(HumanBodyBones.LeftFoot).localPosition.magnitude / 3 * 1.5f;
    }

    private Vector3 TriangleNormal(int a, int b, int c)
    {
        return this.TriangleNormal(this.avatarBonePoint[a].Pos3D, this.avatarBonePoint[b].Pos3D, this.avatarBonePoint[c].Pos3D);
    }

    private Vector3 TriangleNormal(Vector3 a, Vector3 b, Vector3 c)
    {
        Vector3 vector = a - c;
        Vector3 vector2 = Vector3.Cross(a - b, vector);
        vector2.Normalize();
        return vector2;
    }

    private void LateUpdate()
    {
        if (this.ApplyMotion)
        {
            if (this.NatMLRunner == null)
            {
                this.NatMLRunner = GameObject.Find("NatMLRunner").GetComponent<MoveNetSample>();
            }
            else if ((this.AvatarAnimator != null) && this.NatMLRunner.IsPoseUpdate())
            {
                KeyPoint[] keyPoint = this.NatMLRunner.GetKeyPoint();
                this.PoseUpdate(keyPoint);
                footik();
            }

        }
    }

    private Vector3 Vector(int a, int b)
    {
        return (this.avatarBonePoint[a].Pos3D - this.avatarBonePoint[b].Pos3D);
    }

    private Vector3 Vector(Vector3 a, Vector3 b)
    {
        return (a - b);
    }

    // Properties
    public string ErrorMessage { get; private set; }

    float calAngle(float a, float b, float c)
    {
        if (a + b < c)
            return 180;
        else
            return Mathf.Rad2Deg * MathF.Acos((a * a + b * b - c * c) / (2 * a * b));
    }

    void twoBoneIK(Transform bone1, Transform bone2, Transform bone3, Vector3 targetPosition)
    {
        Vector3 startPos = bone1.position;
        Vector3 midPos = bone2.position;
        Vector3 endPos = bone3.position;
        //rightFootGoal.position = targetPosition;

        float a = (midPos - startPos).magnitude;
        float b = (midPos - endPos).magnitude;
        float c = (endPos - startPos).magnitude;
        float c_ = (targetPosition - startPos).magnitude;

        // 调整膝盖部分
        float angleB = calAngle(a, b, c);
        float angleB_ = calAngle(a, b, c_);
        //Debug.LogFormat("angle1: {0}, angle2: {1}", angleB, angleB_);
        Vector3 newBone3 = midPos + Vector3.RotateTowards(endPos - midPos, startPos - midPos, (angleB - angleB_) * Mathf.Deg2Rad, 0.0f);
        Quaternion midRot = Quaternion.FromToRotation(bone3.position - midPos, newBone3 - midPos);
        //Debug.LogFormat("angleOld: {0}", Vector3.Angle(bone3.position - bone2.position, bone1.position - bone2.position));
        bone2.rotation = midRot * bone2.rotation;
        //Debug.LogFormat("angleNew: {0}", Vector3.Angle(bone3.position - bone2.position, bone1.position - bone2.position));

        //调整大腿根部分
        Quaternion midRot_ = Quaternion.FromToRotation(newBone3 - startPos, targetPosition - startPos);
        bone1.rotation = midRot_ * bone1.rotation;

        //Debug.Log("-----------");
    }

    void footik()
    {

        //　IKを使わない場合はこれ以降なにもしない
        if (!useIK)
        {
            return;
        }

        Transform root = AvatarAnimator.GetBoneTransform(HumanBodyBones.Hips);
        Transform leftHip = AvatarAnimator.GetBoneTransform(HumanBodyBones.LeftUpperLeg);
        Transform leftKnee = AvatarAnimator.GetBoneTransform(HumanBodyBones.LeftLowerLeg);
        Transform leftFoot = AvatarAnimator.GetBoneTransform(HumanBodyBones.LeftFoot);
        Transform leftToe = AvatarAnimator.GetBoneTransform(HumanBodyBones.LeftToes);
        Transform rightHip = AvatarAnimator.GetBoneTransform(HumanBodyBones.RightUpperLeg);
        Transform rightKnee = AvatarAnimator.GetBoneTransform(HumanBodyBones.RightLowerLeg);
        Transform rightFoot = AvatarAnimator.GetBoneTransform(HumanBodyBones.RightFoot);
        Transform rightToe = AvatarAnimator.GetBoneTransform(HumanBodyBones.RightToes);

        Vector3 leftFootOffset = Vector3.zero;
        Vector3 rightFootOffsset = Vector3.zero;
        Vector3 leftFootTarget = Vector3.zero;
        Vector3 rightFootTarget = Vector3.zero;

        Vector3 leftRayStart;
        if (leftFoot.position.y < leftToe.position.y)
        {
            leftRayStart = leftFoot.position;
        }
        else
        {
            leftRayStart = leftToe.position;
            leftFootOffset = leftFoot.position - leftToe.position;
        }

        Vector3 rightRayStart;
        if (rightFoot.position.y < rightToe.position.y)
        {
            rightRayStart = rightFoot.position;
        }
        else
        {
            rightRayStart = rightToe.position;
            rightFootOffsset = rightFoot.position - rightToe.position;
        }

        RaycastHit hit;
        //　左足用のレイを飛ばす処理
        var ray = new Ray(leftRayStart + rayPositionOffset, -transform.up);
        //　左足用のレイの視覚化
        Debug.DrawRay(leftRayStart + rayPositionOffset, -transform.up * rayRange, Color.red);
        if (Physics.Raycast(ray, out hit, rayRange))
        {
            leftFootTarget = hit.point + new Vector3(0f, HeelOffset, 0f) + leftFootOffset;
            prevPelvis = leftFootTarget - prevLeftFoot;
            // 先利用hip大概矫正到地面
            if (leftFoot.position.y < leftFootTarget.y)
            {
                root.position = root.position + new Vector3(0, leftFootTarget.y - leftFoot.position.y, 0);
            }
            // 双腿IK，将它矫正到地面
            twoBoneIK(leftHip, leftKnee, leftFoot, leftFootTarget);
            prevLeftFoot = leftFoot.position;
            // 如果腿不够长,伸不到地面
            if (leftFoot.position.y > leftFootTarget.y)
            {
                root.position = root.position - new Vector3(0, leftFoot.position.y - leftFootTarget.y, 0);
            }
        }

        //　右足用のレイの視覚化
        Debug.DrawRay(rightRayStart + rayPositionOffset, -transform.up * rayRange, Color.red);
        //　右足用のレイを飛ばす処理
        ray = new Ray(rightRayStart + rayPositionOffset, -transform.up);
        if (Physics.Raycast(ray, out hit, rayRange, layerMask))
        {
            rightFootTarget = hit.point + new Vector3(0f, HeelOffset, 0f) + rightFootOffsset;
            prevPelvis = prevPelvis + (rightFootTarget - prevRightFoot);
            // 先利用hip大概矫正到地面
            //if (rightFoot.position.y < rightFootTarget.y)
            //{
            //    root.position = root.position + new Vector3(0, rightFootTarget.y - rightFoot.position.y, 0);
            //}
            twoBoneIK(rightHip, rightKnee, rightFoot, rightFootTarget);
            prevRightFoot = rightFoot.position;
            // 如果腿不够长,伸不到地面
            if (rightFoot.position.y > rightFootTarget.y)
            {
                root.position = root.position - new Vector3(0, rightFoot.position.y - rightFootTarget.y, 0);
            }
        }
        //AvatarAnimator.GetBoneTransform(HumanBodyBones.Hips).position = AvatarAnimator.GetBoneTransform(HumanBodyBones.Hips).position + new Vector3(prevPelvis.x / 2.0f, 0, prevPelvis.z / 2.0f);


    }

}
