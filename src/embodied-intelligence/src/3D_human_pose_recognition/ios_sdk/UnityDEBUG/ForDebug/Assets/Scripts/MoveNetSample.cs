/* 
*   MoveNet
*   Copyright (c) 2022 NatML Inc. All Rights Reserved.
*/

namespace NatML.Examples {

    using UnityEngine;
    using UnityEngine.UI;
    using System.Collections.Generic;
    using System.Threading.Tasks;

    public class MoveNetSample : MonoBehaviour {

        [Header("rotate image(0/-1/1):no/counterclockwise/clockwise")]
        public int clockwise = 0;

        
        private Texture2D input1;
        private Texture2D input4;
        private Texture2D input7;
        private float[] heatMap2D = new float[0x4980];
        private float[] heatMap3D = new float[0x80a00];

        private float[] offset2D = new float[0x9300];
        private float[] offset3D = new float[0x181e00];
        private List<OneEuroFilter> oneEuroFilter2D = new List<OneEuroFilter>();
        private List<OneEuroFilter> oneEuroFilter3D = new List<OneEuroFilter>();

        public float OneEuroFilterBeta = 0.005f;
        public float OneEuroFilterDCutoff = 1.2f;
        [Header("One euro filter settings")]
        public bool OneEuroFilterEnable = true;
        public float OneEuroFilterMinCutoff = 3.5f;
        private const float unit = 0.03571429f;

        // constant variable
        private const int InputImageSize = 448;

        
        private bool isPoseUpdate;
        private const int JointNum = 0x18;
        private const int JointNum_Cube = 0x48;
        private const int JointNum_Squared = 0x30;
        private List<KalmanFilter> kalmanFilter2D = new List<KalmanFilter>();
        private List<KalmanFilter> kalmanFilter3D = new List<KalmanFilter>();
        [Header("Kalman filter settings")]
        public bool KalmanFilterEnable = true;
        public float KalmanFilterNoise = 0.4f;
        public float KalmanFilterTimeInterval = 0.45f;
        private KeyPoint[] keyPoints;
        //[SerializeField]
        private bool Lock;
        private List<LowPassFilter> lowPassFilter2D = new List<LowPassFilter>();
        private List<LowPassFilter> lowPassFilter3D = new List<LowPassFilter>();
        [Header("Low pass filter settings")]
        public bool LowPassFilterEnable = true;
        public int LowPassFilterNOrder = 7;
        public float LowPassFilterSmooth = 0.9f;

        public VideoCapture videoCapture;
        private Texture2D emptyTex2D;
        private Texture2D rotatedTexture;

        public KeyPoint[] GetKeyPoint()
        {
            return this.keyPoints;
        }

        private void Init()
        {
            this.keyPoints = new KeyPoint[0x18];
            for (int i = 0; i < 0x18; i++)
            {
                this.keyPoints[i] = new KeyPoint();
                this.keyPoints[i].Index = i;
                this.keyPoints[i].Score3D = 0f;
                this.keyPoints[i].Score2D = 0f;
            }
            this.InitKalmanFilter();
            this.InitLowPassFilter();
            this.InitOneEuroFilter();
        }

        private void InitKalmanFilter()
        {
            this.kalmanFilter2D.Clear();
            this.kalmanFilter3D.Clear();
            for (int i = 0; i < 0x18; i++)
            {
                this.kalmanFilter2D.Add(new KalmanFilter(this.KalmanFilterTimeInterval, this.KalmanFilterNoise));
                this.kalmanFilter3D.Add(new KalmanFilter(this.KalmanFilterTimeInterval, this.KalmanFilterNoise));
            }
        }

        private void InitLowPassFilter()
        {
            this.lowPassFilter2D.Clear();
            this.lowPassFilter3D.Clear();
            for (int i = 0; i < 0x18; i++)
            {
                this.lowPassFilter2D.Add(new LowPassFilter(this.LowPassFilterNOrder, this.LowPassFilterSmooth));
                this.lowPassFilter3D.Add(new LowPassFilter(this.LowPassFilterNOrder, this.LowPassFilterSmooth));
            }
        }

        private void InitOneEuroFilter()
        {
            this.oneEuroFilter2D.Clear();
            this.oneEuroFilter3D.Clear();
            for (int i = 0; i < 0x18; i++)
            {
                this.oneEuroFilter2D.Add(new OneEuroFilter(this.OneEuroFilterMinCutoff, this.OneEuroFilterBeta, this.OneEuroFilterDCutoff));
                this.oneEuroFilter3D.Add(new OneEuroFilter(this.OneEuroFilterMinCutoff, this.OneEuroFilterBeta, this.OneEuroFilterDCutoff));
            }
        }

        public bool IsPoseUpdate()
        {
            return this.isPoseUpdate;
        }

        void DrawPred(Vector3[] pred3D)
        {
            int[] parent = new int[] { 14, 0, 1, 2, 2, 14, 5, 6, 7, 7, 11, 14, 13, 14, 23, 23, 15, 16, 17, 23, 19, 20, 21, -1 }; // 24个关节
            for (int i = 0; i < 24; i++)
                if (parent[i] != -1)
                {
                    //Debug.DrawLine(pred3D[i], pred3D[parent[i]], Color.blue);
                    if (i == 0 || i == 22 || i == 23)  // spine，左拇指，右拇指
                        Debug.DrawLine(pred3D[i] - pred3D[23], pred3D[parent[i]] - pred3D[23], Color.blue);
                    else if ((i >= 0 && i <= 4) || (i >= 15 && i <= 18))  // 右半身
                        Debug.DrawLine(pred3D[i] - pred3D[23], pred3D[parent[i]] - pred3D[23], Color.red);
                    else
                        Debug.DrawLine(pred3D[i] - pred3D[23], pred3D[parent[i]] - pred3D[23], Color.green);
                }
        }

        public List<Vector4> ReadPose(string poseText)
        {
            string[] spitRes = poseText.Split(',');         
            List<Vector4> ret = new List<Vector4>();
            for (int i = 0; i < 24; i++)
            {
                ret.Add(new Vector4(float.Parse(spitRes[i * 3]), float.Parse(spitRes[i * 3 + 1]), float.Parse(spitRes[i * 3 + 2]), float.Parse(spitRes[24*3+i])));
            }
            return ret;
        }

        private void PredictPose3D()
        {
            this.Score3D = 0f;
            float oefTime = 0.02f;// Time.time;
            Vector3[] jpPoint = new Vector3[24];
            List<Vector4> point_score = ReadPose("10.755615, 5.0601196, 13.379883, 9.739746, 6.6589355, 15.388672, 10.611328, 6.4819336, 17.441895, 11.654053, 6.3791504, 16.930908, 12.106628, 6.378174, 16.78711, 15.172485, 4.7945557, 13.5563965, 14.993164, 6.885254, 15.188599, 15.474609, 9.015137, 14.438477, 14.502441, 9.01123, 14.685303, 15.536621, 10.003906, 14.175781, 12.759399, 2.836792, 14.754028, 11.719238, 2.9431152, 14.56958, 11.873047, 3.1151733, 13.004166, 10.764648, 3.045868, 14.278076, 11.36377, 3.4135742, 14.2109375, 13.26709, 11.120483, 11.939423, 12.882141, 15.288574, 12.161743, 14.206909, 19.48584, 10.434082, 12.478516, 20.507812, 11.236816, 14.364502, 11.034302, 14.100159, 14.062805, 15.231201, 15.263184, 17.33545, 19.24231, 14.695801, 15.455078, 20.182861, 17.053497, 14.184448, 9.548584, 12.50293, 0.8071289, 0.5996094, 0.5839844, 0.56933594, 0.5283203, 0.7915039, 0.37451172, 0.13769531, 0.15771484, 0.15136719, 0.9135742, 0.8198242, 0.9477539, 0.9550781, 0.7885742, 0.9511719, 0.89501953, 0.6777344, 0.59228516, 0.87597656, 0.9145508, 0.70458984, 0.70458984, 0.7968752");
            Debug.Log("pred3d");
            #region 一维处理
            Parallel.For(0, 24, delegate (int j)
            {
                KeyPoint point = this.keyPoints[j];      
                
                point.Now3D = new Vector3(point_score[j].x, point_score[j].y, point_score[j].z);
                point.Score3D = point_score[j].w;                

                if (clockwise==0)
                    point.Pos3D = new Vector3( -point.Now3D.x, -point.Now3D.y, point.Now3D.z); //不旋转
                else if (clockwise == -1)
                    point.Pos3D = new Vector3(-point.Now3D.y, point.Now3D.x, point.Now3D.z); //逆时针旋转
                else if(clockwise == 1)
                    point.Pos3D = new Vector3(point.Now3D.y, -point.Now3D.x, point.Now3D.z); //顺时针旋转

                if (this.KalmanFilterEnable)
                {
                    point.Pos3D = this.kalmanFilter3D[j].CorrectAndPredict(point.Pos3D);
                }
                if (this.LowPassFilterEnable)
                {
                    point.Pos3D = this.lowPassFilter3D[j].CorrectAndPredict(point.Pos3D);
                }
                if (this.OneEuroFilterEnable)
                {
                    point.Pos3D = this.oneEuroFilter3D[j].CorrectAndPredict(point.Pos3D, oefTime);
                }
                jpPoint[j] = point.Pos3D;
            });
            #endregion
            this.Score3D /= 24f;
            this.isPoseUpdate = true;
            DrawPred(jpPoint);
        }

        public void SetKalmanFilter(bool enabled)
        {
            if (this.KalmanFilterEnable != enabled)
            {
                this.InitKalmanFilter();
            }
            this.KalmanFilterEnable = enabled;
        }

        public void SetKalmanFilterParameter(float timeInterval, float noise)
        {
            this.KalmanFilterTimeInterval = timeInterval;
            this.KalmanFilterNoise = noise;
            this.InitKalmanFilter();
        }

        public void SetLowPassFilter(bool enabled)
        {
            if (this.LowPassFilterEnable != enabled)
            {
                this.InitLowPassFilter();
            }
            this.LowPassFilterEnable = enabled;
        }

        public void SetLowPassFilterParameter(int nOrder, float smooth)
        {
            if (nOrder <= 0)
            {
                nOrder = 1;
            }
            else if (nOrder >= 10)
            {
                nOrder = 10;
            }
            this.LowPassFilterNOrder = nOrder;
            this.LowPassFilterSmooth = smooth;
            this.InitLowPassFilter();
        }

        public void SetOneEuroFilter(bool enabled)
        {
            if (this.OneEuroFilterEnable != enabled)
            {
                this.InitOneEuroFilter();
            }
            this.OneEuroFilterEnable = enabled;
        }

        public void SetOneEuroFilterParameter(float minCutoff, float beta, float dCutoff)
        {
            this.OneEuroFilterMinCutoff = minCutoff;
            this.OneEuroFilterBeta = beta;
            this.OneEuroFilterDCutoff = dCutoff;
            this.InitOneEuroFilter();
        }


        void Start () {
            this.Init();
        }

        void Update () {
            PredictPose3D();
        }

        // Properties
        [HideInInspector]
        public float Score2D { get; private set; }

        [HideInInspector]
        public float Score3D { get; private set; }
    }
}