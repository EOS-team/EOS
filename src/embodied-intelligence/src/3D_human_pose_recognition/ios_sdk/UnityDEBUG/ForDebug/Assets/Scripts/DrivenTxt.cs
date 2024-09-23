using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using UnityEngine;

public class DrivenTxt : MonoBehaviour
{
    // Start is called before the first frame update

    private AvatarBonePoint[] avatarBonePoint;
    private Animator animator;
    private List<Quaternion> rotations;


    public List<Quaternion> ReadTxTContent(string contentPath)
    {
        string[] spitRes = new string[128];
        if (File.Exists(contentPath))
        {
              
            using (StreamReader sr = new StreamReader(contentPath))
            {
                string line;                    
                // 从文件读取并显示行，直到文件的末尾 
                while ((line = sr.ReadLine()) != null)
                {
                    spitRes = line.Split(',');
                }
            }                
        }        

        List<Quaternion> ret = new List<Quaternion>();
        for (int i = 0;i< spitRes.Length/4;i++)
        {
            ret.Add(new Quaternion(float.Parse(spitRes[i * 4]), float.Parse(spitRes[i * 4 + 1]), float.Parse(spitRes[i * 4 + 2]), float.Parse(spitRes[i * 4 + 3])));
        }
        return ret;
    }
    void Start()
    {
        this.avatarBonePoint = new AvatarBonePoint[32];
        this.animator = GetComponent<Animator>();
        for (int i = 0; i < 32; i++)
        {
            this.avatarBonePoint[i] = new AvatarBonePoint();
            this.avatarBonePoint[i].Enabled = true;
            this.avatarBonePoint[i].Lock = false;
            this.avatarBonePoint[i].Error = 0;
        }
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

       rotations = ReadTxTContent(@"E:\Code\python\TDPTAPP\Swift\res_quat.txt");
    }
        

    // Update is called once per frame
    void Update()
    {
       
        if(rotations.Count == 32)
        {
            for(int i=0; i < 32; i++)
            {
                if (this.avatarBonePoint[i].Transform == null) continue;
                this.avatarBonePoint[i].Transform.rotation = rotations[i];
            }
            Debug.Log("finish");
        }
        else
        {
            Debug.Log("length is not correct, the result is not 32");
        }
    }
}
