using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Controls;
using System.Windows.Shapes;
using VisibilityIn2DGrid.Helper.UI.Constants;
using VisibilityIn2DGrid.Index;

namespace VisibilityIn2DGrid.Helper.UI;

public class CanvasManager
{
    private readonly Canvas _canvas;
    private readonly ScrollViewer _scrollViewer;
    private readonly List<Polygon> _obstacles = [];
    private readonly List<Line> _visualizationRays = [];

    private SpatialIndex? _spatialIndex;

    public CanvasManager(Canvas canvas, ScrollViewer scrollViewer)
    {
        _canvas = canvas;
        _scrollViewer = scrollViewer;

        _canvas.Width = ViewConstants.CanvasWidth;
        _canvas.Height = ViewConstants.CanvasHeight;
    }

    public void DrawGrid()
    {
        GridLineGenerator.DrawMajorGridLines(_canvas, ViewConstants.CanvasWidth, ViewConstants.CanvasHeight,
            ViewConstants.GridSize, ViewConstants.GridLineThickness);
    }

    public void ClearObstacles()
    {
        foreach (var child in _obstacles)
        {
            _canvas.Children.Remove(child);
        }
        _obstacles.Clear();
    }

    public void AddRandomPolygons(int count = 500)
    {
        ClearObstacles();

        for (int i = 0; i < count; i++)
        {
            var polygon = RandomPolygonGenerator.GenerateRandomPolygon(
                ViewConstants.CanvasWidth, ViewConstants.CanvasHeight);

            _obstacles.Add(polygon);
            _canvas.Children.Add(polygon);
        }
    }

    public void InitializeSpatialIndex()
    {
        _spatialIndex?.Dispose();
        _spatialIndex = new SpatialIndex(
            (float)ViewConstants.CanvasWidth,
            (float)ViewConstants.CanvasHeight,
            ViewConstants.GridSize);

        foreach (var obstacle in _obstacles)
        {
            _spatialIndex.Insert(obstacle);
        }
    }

    public void CenterCanvas()
    {
        if (_scrollViewer == null) return;

        double horizontalOffset = (ViewConstants.CanvasWidth - _scrollViewer.ViewportWidth) / 2;
        double verticalOffset = (ViewConstants.CanvasHeight - _scrollViewer.ViewportHeight) / 2;

        _scrollViewer.ScrollToHorizontalOffset(horizontalOffset);
        _scrollViewer.ScrollToVerticalOffset(verticalOffset);
    }

    public void ClearVisualizationRays()
    {
        foreach (var ray in _visualizationRays)
        {
            _canvas.Children.Remove(ray);
        }
        _visualizationRays.Clear();
    }

    public void AddVisualizationRay(Line ray)
    {
        _visualizationRays.Add(ray);
        _canvas.Children.Add(ray);
    }

    public SpatialIndex? GetSpatialIndex() => _spatialIndex;
    public List<Polygon> GetObstacles() => _obstacles;
}
