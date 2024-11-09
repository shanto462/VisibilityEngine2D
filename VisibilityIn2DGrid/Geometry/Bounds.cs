using System.Numerics;

namespace VisibilityEngine2D.Geometry;

public class Bounds
{
    public Vector2 Min { get; set; }
    public Vector2 Max { get; set; }

    public Bounds(Vector2 min, Vector2 max)
    {
        Min = min;
        Max = max;
    }

    public Bounds(float minX, float minY, float maxX, float maxY)
    {
        Min = new Vector2(minX, minY);
        Max = new Vector2(maxX, maxY);
    }

    public bool Intersects(Bounds other)
    {
        return !(Max.X < other.Min.X || Min.X > other.Max.X ||
                Max.Y < other.Min.Y || Min.Y > other.Max.Y);
    }
}