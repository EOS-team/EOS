using NUnit.Framework;
using UnityEditor.Performance.ProfileAnalyzer;

public class ProfileDataTests
{
    [Test]
    public void Save_WithNullData_ReturnsFalse()
    {
        var filename = "filename.pdata";
        ProfileData nullProfileData = null;

        bool success = ProfileData.Save(filename, nullProfileData);

        Assert.IsFalse(success, "Calling ProfileData.Save with null data should return false.");
    }

    [Test]
    public void Save_WithNullFilename_ReturnsFalse()
    {
        string filename = null;
        var profileData = new ProfileData();

        bool success = ProfileData.Save(filename, profileData);

        Assert.IsFalse(success, "Calling ProfileData.Save with a null filename should return false.");
    }

    [Test]
    public void Save_WithEmptyFilename_ReturnsFalse()
    {
        string filename = string.Empty;
        var profileData = new ProfileData();

        bool success = ProfileData.Save(filename, profileData);

        Assert.IsFalse(success, "Calling ProfileData.Save with an empty filename should return false.");
    }
}
