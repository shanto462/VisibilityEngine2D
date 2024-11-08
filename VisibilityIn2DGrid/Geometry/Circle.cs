﻿using System.Numerics;
using System.Windows;
using System.Windows.Shapes;
using VisibilityIn2DGrid.Extensions;

namespace VisibilityIn2DGrid.Geometry;

public class Circle
{
    public Point Center { get; set; }
    public float Radius { get; set; }

    public Circle(Point center, float radius)
    {
        Center = center;
        Radius = radius;
    }

    public Circle(float x, float y, float radius)
    {
        Center = new Point(x, y);
        Radius = radius;
    }

    public Bounds GetBounds()
    {
        return new Bounds(
            (float)(Center.X - Radius),
            (float)(Center.Y - Radius),
            (float)(Center.X + Radius),
            (float)(Center.Y + Radius)
        );
    }

    public bool ContainsPoint(Point point)
    {
        double dx = point.X - Center.X;
        double dy = point.Y - Center.Y;
        return dx * dx + dy * dy <= Radius * Radius;
    }

    public bool IntersectsPolygon(Polygon polygon)
    {
        if (!GetBounds().Intersects(polygon.GetBounds()))
            return false;

        foreach (var point in polygon.Points)
        {
            if (ContainsPoint(point))
                return true;
        }

        for (int i = 0; i < polygon.Points.Count; i++)
        {
            var p1 = polygon.Points[i];
            var p2 = polygon.Points[(i + 1) % polygon.Points.Count];

            if (IntersectsLineSegment(p1, p2))
                return true;
        }

        return polygon.ContainsPoint(Center);
    }

    private bool IntersectsLineSegment(Point p1, Point p2)
    {
        float lineLength = Vector2.Distance(new Vector2((float)p1.X, (float)p1.Y), new Vector2((float)p2.X, (float)p2.Y));

        var t = ((Center.X - p1.X) * (p2.X - p1.X) + (Center.Y - p1.Y) * (p2.Y - p1.Y)) / (lineLength * lineLength);
        t = Math.Max(0, Math.Min(1, t));

        var closestX = p1.X + t * (p2.X - p1.X);
        var closestY = p1.Y + t * (p2.Y - p1.Y);

        var dx = closestX - Center.X;
        var dy = closestY - Center.Y;

        return dx * dx + dy * dy <= Radius * Radius;
    }
}