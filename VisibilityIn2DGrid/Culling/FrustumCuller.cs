using System.Windows;
using System.Windows.Shapes;

namespace VisibilityEngine2D.Culling;

public static class FrustumCuller
{
    public class ViewFrustum(Point center, double fovAngle, double direction, double range)
    {
        public Point Center { get; set; } = center;
        public double FOVAngle { get; set; } = fovAngle;
        public double Direction { get; set; } = direction;
        public double Range { get; set; } = range;

        public (Point, Point, Point) GetFrustumLines()
        {
            double startAngle = (Direction - (FOVAngle / 2)) * Math.PI / 180;
            double endAngle = (Direction + (FOVAngle / 2)) * Math.PI / 180;

            Point leftPoint = new(
                Center.X + (Range * Math.Cos(startAngle)),
                Center.Y + (Range * Math.Sin(startAngle))
            );

            Point rightPoint = new(
                Center.X + (Range * Math.Cos(endAngle)),
                Center.Y + (Range * Math.Sin(endAngle))
            );

            return (Center, leftPoint, rightPoint);
        }
    }

    private static bool IsPointInFrustum(Point point, ViewFrustum frustum)
    {
        (_, _, _) = frustum.GetFrustumLines();

        double distanceToCenter = Math.Sqrt(
            Math.Pow(point.X - frustum.Center.X, 2) +
            Math.Pow(point.Y - frustum.Center.Y, 2)
        );
        if (distanceToCenter > frustum.Range)
        {
            return false;
        }

        double angleToPoint = Math.Atan2(
            point.Y - frustum.Center.Y,
            point.X - frustum.Center.X
        ) * 180 / Math.PI;

        double normalizedDirection = frustum.Direction % 360;
        if (normalizedDirection < 0)
        {
            normalizedDirection += 360;
        }

        double normalizedAngle = angleToPoint % 360;
        if (normalizedAngle < 0)
        {
            normalizedAngle += 360;
        }

        double halfFOV = frustum.FOVAngle / 2;
        double angleDiff = Math.Abs(normalizedAngle - normalizedDirection);
        if (angleDiff > 180)
        {
            angleDiff = 360 - angleDiff;
        }

        return angleDiff <= halfFOV;
    }

    public static bool IsPolygonVisible(Polygon polygon, ViewFrustum frustum)
    {
        List<Point> points = polygon.Points.Select(p => new Point(p.X, p.Y)).ToList();

        foreach (Point point in points)
        {
            if (IsPointInFrustum(point, frustum))
            {
                return true;
            }
        }

        if (IsPointInPolygon(frustum.Center, points))
        {
            return true;
        }

        (Point center, Point left, Point right) = frustum.GetFrustumLines();
        List<(Point, Point)> frustumLines =
        [
            (center, left),
            (center, right)
        ];

        for (int i = 0; i < points.Count; i++)
        {
            Point start = points[i];
            Point end = points[(i + 1) % points.Count];

            foreach ((Point frustumStart, Point frustumEnd) in frustumLines)
            {
                if (DoLinesIntersect(start, end, frustumStart, frustumEnd))
                {
                    return true;
                }
            }
        }

        return false;
    }

    private static bool IsPointInPolygon(Point point, List<Point> polygonPoints)
    {
        bool inside = false;
        for (int i = 0, j = polygonPoints.Count - 1; i < polygonPoints.Count; j = i++)
        {
            if (((polygonPoints[i].Y > point.Y) != (polygonPoints[j].Y > point.Y)) &&
                (point.X < ((polygonPoints[j].X - polygonPoints[i].X) * (point.Y - polygonPoints[i].Y) /
                (polygonPoints[j].Y - polygonPoints[i].Y)) + polygonPoints[i].X))
            {
                inside = !inside;
            }
        }
        return inside;
    }

    private static bool DoLinesIntersect(Point p1, Point p2, Point p3, Point p4)
    {
        double den = ((p4.Y - p3.Y) * (p2.X - p1.X)) - ((p4.X - p3.X) * (p2.Y - p1.Y));

        if (den == 0)
        {
            return false;
        }

        double ua = (((p4.X - p3.X) * (p1.Y - p3.Y)) - ((p4.Y - p3.Y) * (p1.X - p3.X))) / den;
        double ub = (((p2.X - p1.X) * (p1.Y - p3.Y)) - ((p2.Y - p1.Y) * (p1.X - p3.X))) / den;

        return ua >= 0 && ua <= 1 && ub >= 0 && ub <= 1;
    }

    public static List<Polygon> GetVisiblePolygons(IList<Polygon> polygons, ViewFrustum frustum)
    {
        return polygons.Where(polygon => IsPolygonVisible(polygon, frustum)).ToList();
    }
}
