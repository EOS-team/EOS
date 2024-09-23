using UnityEngine;
using UnityEngine.Experimental.Rendering;
using UnityEngine.UI;
using UnityEngine.Video;

public class VideoCapture : MonoBehaviour
{
    public GameObject InputTexture;
    public RawImage VideoScreen;
    public GameObject VideoBackground;
    public float VideoBackgroundScale;
    public LayerMask _layer;
    public bool UseWebCam = true;
    public int WebCamIndex = 0;
    public VideoPlayer VideoPlayer;

    private WebCamTexture webCamTexture;
    private RenderTexture videoTexture;
    public Texture2D sampleImg;

    private int videoScreenWidth = 2560;
    private int bgWidth, bgHeight;
    private readonly int CameraWidth = 1280;
    private readonly int CameraHeight = 720;
    private readonly int CameraFps = 60;

    public RenderTexture MainTexture { get; private set; }
    public Texture2D MainTexture2D { get; private set; }

    /// <summary>
    /// Initialize Camera
    /// </summary>
    /// <param name="bgWidth"></param>
    /// <param name="bgHeight"></param>
    public void Init(int bgWidth, int bgHeight)
    {
        this.bgWidth = bgWidth;
        this.bgHeight = bgHeight;
        if (UseWebCam) CameraPlayStart();
        else VideoPlayStart();
    }

    /// <summary>
    /// Play Web Camera
    /// </summary>
    private void CameraPlayStart()
    {
        WebCamDevice[] devices = WebCamTexture.devices;
#if UNITY_EDITOR
        webCamTexture = new WebCamTexture(devices[0].name);
#elif UNITY_IPHONE
        webCamTexture = new WebCamTexture(devices[1].name);
#else
        webCamTexture = new WebCamTexture(devices[0].name);
#endif

        var sd = VideoScreen.GetComponent<RectTransform>();
        VideoScreen.texture = webCamTexture;

        webCamTexture.Play();

        sd.sizeDelta = new Vector2(videoScreenWidth, videoScreenWidth * webCamTexture.height / webCamTexture.width);
        var aspect = (float)webCamTexture.width / webCamTexture.height;
        VideoBackground.transform.localScale = new Vector3(aspect, 1, 1) * VideoBackgroundScale;
        VideoBackground.GetComponent<Renderer>().material.mainTexture = webCamTexture;

        InitMainTexture();
    }

    /// <summary>
    /// Play video
    /// </summary>
    private void VideoPlayStart()
    {
        videoTexture = new RenderTexture((int)VideoPlayer.clip.width, (int)VideoPlayer.clip.height, 24);

        VideoPlayer.renderMode = VideoRenderMode.RenderTexture;
        VideoPlayer.targetTexture = videoTexture;

        var sd = VideoScreen.GetComponent<RectTransform>();
        sd.sizeDelta = new Vector2(videoScreenWidth, (int)(videoScreenWidth * VideoPlayer.clip.height / VideoPlayer.clip.width));
        VideoScreen.texture = videoTexture;

        VideoPlayer.Play();
        Debug.Log("play video");

        var aspect = (float)videoTexture.width / videoTexture.height;

        VideoBackground.transform.localScale = new Vector3(aspect, 1, 1) * VideoBackgroundScale;
        VideoBackground.GetComponent<Renderer>().material.mainTexture = videoTexture;

        InitMainTexture();
    }

    /// <summary>
    /// Initialize Main Texture
    /// </summary>
    private void InitMainTexture()
    {
        GameObject go = new GameObject("MainTextureCamera", typeof(Camera));

        go.transform.parent = VideoBackground.transform;
        go.transform.localScale = new Vector3(-1.0f, -1.0f, 1.0f);
        go.transform.localPosition = new Vector3(0.0f, 0.0f, -2.0f);
        go.transform.localEulerAngles = Vector3.zero;
        go.layer = _layer;

        var camera = go.GetComponent<Camera>();
        camera.orthographic = true;
        camera.orthographicSize = 0.5f ;
        camera.depth = -5;
        camera.depthTextureMode = 0;
        camera.clearFlags = CameraClearFlags.Color;
        camera.backgroundColor = Color.black;
        camera.cullingMask = _layer;
        camera.useOcclusionCulling = false;
        camera.nearClipPlane = 1.0f;
        camera.farClipPlane = 5.0f;
        camera.allowMSAA = false;
        camera.allowHDR = false;
#if UNITY_IPHONE
        if (UseWebCam)
                go.transform.localRotation = Quaternion.Euler(0.0f,0.0f,90.0f);
#endif

        MainTexture = new RenderTexture(bgWidth, bgHeight, 0, RenderTextureFormat.ARGB32, RenderTextureReadWrite.sRGB)
        {
            useMipMap = false,
            autoGenerateMips = false,
            wrapMode = TextureWrapMode.Clamp,
            filterMode = FilterMode.Point,
            antiAliasing = 1,
        };

        camera.targetTexture = MainTexture;
        if (InputTexture.activeSelf) InputTexture.GetComponent<Renderer>().material.mainTexture = MainTexture;
    }
}


public static class ExtensionMethod
{
    public static Texture2D toTexture2D(this RenderTexture rt,Texture2D emptyTex2D)
    {
        //Texture2D texture = new Texture2D(rt.width, rt.height, rt.graphicsFormat, 0, TextureCreationFlags.None);

        RenderTexture.active = rt;
        emptyTex2D.ReadPixels(new Rect(0, 0, rt.width, rt.height), 0, 0);
        emptyTex2D.Apply();
        RenderTexture.active = null;

        return emptyTex2D;
    }
}